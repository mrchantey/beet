//! The `beet` binary: discover an entry, supply the process request, load it,
//! let the loaded tree run itself, and exit unless something kept it alive.
//!
//! beet is unopinionated like a game engine: it links a library of capabilities
//! (registered reflect types) but ships zero behaviour. It discovers `main.bsx`
//! (or `main.json`/`main.ron`) by walking the cwd and its ancestors, consumes
//! only its own `--main` flag, and builds the entry on the async runtime through
//! its [`BlobStore`] (so every store read is awaited, never blocked), then lets the
//! `BootOnLoad` verb fan the process request out on the build's `LoadTemplate`. A
//! one-shot streams its response and exits; a long-running server parks its boot
//! call to persist the process.
//!
//! The entry load is target-agnostic (the shared `site_build` core reads any
//! [`BlobStore`]); only entry *resolution* differs by target. Native walks the
//! filesystem for `main.bsx` (or honours `--main`) and selects an `fs`/`memory`
//! store; wasm has no filesystem walk, so it requires an explicit `--main` (the same
//! `fs`/`memory` store, the `fs` store reading through the deno runner's fs globals),
//! and is driven by `run_async` (native `run()` busy-waits on the JS event loop). The
//! dev-command, winit-render and remote/S3 paths are native-only.
use beet::prelude::*;
// `ENTRY_NAMES` (the entry-document name list discovery looks for) and `resolve_store`
// (the `--store` backend selector) are shared with the `check`/`serve`/`export-static`
// commands, so they live in the lib's `site_build` module rather than here.
use beet_cli::prelude::*;

// the wasm `beet` binary: the same unopinionated entry load as native, driven by
// `run_async` on the JS event loop (native `run()` busy-waits there and would block
// it). It requires an explicit `--main` + `--store` (default `fs`): there is no
// filesystem ancestor walk to discover an entry, and no winit/dev-command/remote
// surface (all native-only). The entry's own `BootOnLoad`/`CliServer` drives output
// and writes `AppExit`, which `AppExitPlugin` turns into `Deno.exit`.
#[cfg(target_arch = "wasm32")]
fn main() {
	console_error_panic_hook::set_once();
	let mut app = App::new();
	app.add_plugins(BeetPlugins)
		.add_systems(Startup, load_entry);
	// spawn the runner on the JS loop and detach; the Deno runner's `loop_forever`
	// holds the process open until the entry writes `AppExit`.
	async_ext::spawn_local(async move {
		let _ = app.run_async().await;
	})
	.detach();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> AppExit {
	// load any local `.env` (eg `BEET_REMOTE_URL`) before the app starts.
	env_ext::load_dotenv();

	App::new()
		.add_plugins((
			// the trusted defaults: the runner (the headless 30Hz loop here), beet's
			// logging, the async runtime, and the router/scene/server + native terminal
			// capabilities, all selected by feature flag.
			BeetPlugins,
			// the windowed render path's window lifecycle + screenshot harness. The
			// facade's `BeetPlugins` links winit windowless (a capability, not a window);
			// the binary owns the lifecycle (continuous updates, escape/close-to-exit,
			// `BEET_SCREENSHOT` capture), so a data-spawned `<Window/>` appears and a
			// headless `.bsx` keeps running under the render binary.
			#[cfg(feature = "winit")]
			render_window_plugin,
			// the native-only dev-command capabilities, linked as registered types and
			// inert until a `main.bsx` names them.
			CliCommandsPlugin,
		))
		.add_systems(
			Startup,
			// the process exits when `boot` writes `AppExit` for the one-shot it
			// resolves; a long-running server parks its boot call, so its unresolved
			// `Running<Response>` persists the process with no refcount
			load_entry,
		)
		.run()
}

/// `Startup`: resolve the entry store + name and build the entry, all on the async
/// runtime so discovery (a store walk), template registration, and every store read
/// (`templates/`, the entry document, `<RoutesDir>`/`<Template src>`) go through the
/// one [`BlobStore`] without ever blocking the runtime (which is single-threaded on
/// wasm). The app loop drives the task; its build fires `LoadTemplate` on the root,
/// where the `BootOnLoad` verb fans the process request out to the entry's servers.
/// The app then stays alive until something writes `AppExit`, so nothing is held by
/// hand here. A failed resolve/build logs and exits with an error rather than
/// panicking. Target-agnostic: native and wasm build the same way, differing only in
/// how [`resolve_entry`] finds the store (a filesystem walk vs an explicit `--main`).
fn load_entry(world: &mut World) {
	// the binary consumes only its own args here; the loaded tree re-parses argv.
	let args = CliArgs::parse_env();
	// the recognized template formats (`.bsx`, `.js`), read once here so the async
	// build can both filter the `templates/` read and lower each source by format.
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	world.run_async_local(async move |world: AsyncWorld| {
		// browser: there is no filesystem and no `--main`; the program is inlined in a
		// `<script type="application/x-bsx">`. Read it from the DOM and build it onto a
		// storeless root through the same core as native, rather than resolving a store.
		#[cfg(target_arch = "wasm32")]
		if js_runtime::environment() == js_runtime::JsEnvironment::Browser {
			if let Err(err) = browser_entry(&world, formats).await {
				error!("{err}");
				world.write_message(AppExit::error()).await;
			}
			return;
		}
		// resolve on the runtime, since discovery now awaits the store.
		let resolved = match resolve_entry(&args).await {
			Ok(resolved) => resolved,
			Err(err) => {
				error!("{err}");
				world.write_message(AppExit::error()).await;
				return;
			}
		};
		if let Err(err) = build_entry(&world, args, resolved, formats).await {
			error!("{err}");
			world.write_message(AppExit::error()).await;
		}
	});
}

/// Build the browser entry: read the program from the DOM via
/// [`MainBsx::read_dom_program`] and build it onto a storeless root (see
/// [`build_entry_from_bsx`]). The wasm `Browser` branch of [`load_entry`]; the
/// program's own `RunOnLoad`/`ExchangeOnLoad` verb then drives it.
#[cfg(target_arch = "wasm32")]
async fn browser_entry(world: &AsyncWorld, formats: TemplateFormats) -> Result {
	let bsx = MainBsx::read_dom_program().await?;
	world
		.with(move |world: &mut World| {
			build_entry_from_bsx(world, formats, "main.bsx", bsx, ())
		})
		.await?;
	Ok(())
}

/// A resolved entry: its store, the entry document name within it, and the local
/// dir to watch for live reload (`None` for a remote entry, and always `None` on
/// wasm, where there is no local-dev watch path).
struct ResolvedEntry {
	store: BlobStore,
	entry_name: String,
	#[cfg(not(target_arch = "wasm32"))]
	watch_dir: Option<AbsPathBuf>,
}

/// Build the resolved entry on the async runtime: register the entry's `templates/`
/// and read the entry document through the store (awaited, not blocked), then build
/// it into a root carrying the store so `<RoutesDir>` and `<Template src>` resolve
/// the store by ancestry. The build fires `LoadTemplate`, where `BootOnLoad` boots
/// the servers. Target-agnostic; the `--watch` live-reload path is native-only.
async fn build_entry(
	world: &AsyncWorld,
	args: CliArgs,
	resolved: ResolvedEntry,
	formats: TemplateFormats,
) -> Result {
	let store = resolved.store;
	let entry_name = resolved.entry_name;
	let sources =
		read_entry_sources(&store, formats, entry_name.clone()).await?;
	// the `--watch` path (native-only) needs the entry dir and the args; on wasm
	// neither is used, so they go unread there.
	#[cfg(not(target_arch = "wasm32"))]
	let watch_dir = resolved.watch_dir;
	#[cfg(target_arch = "wasm32")]
	let _ = (&args, &entry_name);
	world
		.with(move |world: &mut World| -> Result {
			// the binary stays unopinionated: it spawns the entry root with no load
			// verb of its own, so the entry's own markup declares how it loads. A
			// server entry spreads `BootOnLoad` beside its servers, a script entry
			// spreads `ExchangeOnLoad`, a render scene `RunOnLoad`, and a self-booting
			// verb (eg a thread's `{CreateThread}`) `#[require]`s `BootOnLoad` itself.
			// the entry document's own dir, watched for live reload (keyed to the
			// entry store) so editing `main.bsx` itself hot-reloads; computed before
			// `build_entry_root` consumes `store`. Inert on a non-fs store / on wasm.
			#[cfg(not(target_arch = "wasm32"))]
			let entry_watch = WatchDir::for_entry(&store, &entry_name);
			#[cfg(target_arch = "wasm32")]
			let _ = &entry_name;
			let _root = build_entry_root(world, store, sources, ())?;
			// `--watch` (local dev): mark the root for live reload, and watch the entry
			// document's own dir (its `<RoutesDir>`/`<TemplateDir>`/`<AssetsDir>` mounts
			// register their own `WatchDir`s as they resolve). The watchers emit
			// `BlobEvent`s, so editing a template/slide/style/the entry hot-reloads
			// connected browsers, the deck's `<LiveReloadScript/>` widget turning the
			// broadcast into a reload. Opt-in, so a running presentation never reloads
			// underfoot; a deployed (remote) entry has no local dir to watch. Native-only:
			// the wasm runner has no fs watcher.
			#[cfg(not(target_arch = "wasm32"))]
			if watch_dir.is_some() && args.params.contains_key("watch") {
				let mut root_mut = world.entity_mut(_root);
				root_mut.insert(LiveReload::new());
				if let Some(entry_watch) = entry_watch {
					root_mut.insert(entry_watch);
				}
				world.flush();
			}
			Ok(())
		})
		.await
}

/// Resolve the entry [`BlobStore`], the entry document name within it, and the
/// local directory to watch for dev live reload (`None` when there is no local dir,
/// ie a remote entry).
///
/// Resolution order:
/// 1. `BEET_SERVICE_ACCESS=remote` (a deployed task): load from a remote store. The
///    remote-access concept is general, but only a compiled-in backend can serve it
///    (`aws_sdk` → S3/R2); without one this errors rather than falling through.
/// 2. `--main=<path>`: the entry dir is the path's parent, the name its file name,
///    and the store is the `--store` kind rooted at that dir.
/// 3. otherwise: discovery walks the cwd and its ancestors through an `fs` store for
///    the first [`ENTRY_NAMES`] match.
///
/// The `--store` arg selects the backend (default `fs`); see [`resolve_store`] for
/// the supported kinds.
#[cfg(not(target_arch = "wasm32"))]
async fn resolve_entry(args: &CliArgs) -> Result<ResolvedEntry> {
	resolve_entry_native(args).await
}

/// Resolve the entry on wasm: there is no filesystem ancestor walk and no remote
/// backend, so `--main` is required and `--store` (default `fs`) picks the backend.
/// The entry dir is the path's parent and the store is rooted there; descendants
/// resolve it by ancestry just as on native. The `fs` store reads through the deno
/// runner's fs globals (see [`resolve_store`]), so the same on-disk entry loads.
#[cfg(target_arch = "wasm32")]
async fn resolve_entry(args: &CliArgs) -> Result<ResolvedEntry> {
	let entry = args
		.params
		.get("main")
		.map(|path| AbsPathBuf::new(path.as_str()))
		.ok_or_else(|| {
			bevyhow!(
				"the wasm `beet` binary requires an explicit `--main=<path>` (there is \
				no filesystem entry discovery on wasm)"
			)
		})??;
	let (dir, entry_name) = entry_root_and_name(&args.params, &entry)?;
	Ok(ResolvedEntry {
		store: resolve_store(&args.params, dir)?,
		entry_name,
	})
}

/// Resolve the (store root dir, entry name within it) for an explicit `--main`.
///
/// By default the store roots at the **workspace root** (the nearest ancestor with a
/// `Cargo.lock`, or `$WORKSPACE_ROOT`), with the name carrying the entry path
/// relative to it, so an entry reaches workspace paths (eg `assets/`) with no flag.
/// `--root=<dir>` overrides that root explicitly. Either way the entry can reach
/// sibling/ancestor paths (eg the wasm page at `examples/wasm/main.bsx` inlining the
/// workspace-relative `examples/action/hello_world.bsx`), and live reload watches the
/// whole root. Falls back to the entry's own directory when it is not under the
/// resolved root (or on wasm, which has no workspace).
fn entry_root_and_name(
	params: &MultiMap<SmolStr, SmolStr>,
	entry: &AbsPathBuf,
) -> Result<(AbsPathBuf, String)> {
	match params.get("root") {
		// explicit `--root`: the entry must live under it.
		Some(root) => {
			let root = AbsPathBuf::new(root.as_str())?;
			let entry_name = entry_name_under(entry, &root).ok_or_else(|| {
				bevyhow!("--main `{entry}` is not under --root `{root}`")
			})?;
			Ok((root, entry_name))
		}
		// default: the workspace root, falling back to the entry's own dir.
		None => match default_entry_root()
			.and_then(|root| Some((entry_name_under(entry, &root)?, root)))
		{
			Some((entry_name, root)) => Ok((root, entry_name)),
			None => {
				let dir = entry.parent().ok_or_else(|| {
					bevyhow!("entry `{entry}` has no parent directory")
				})?;
				let entry_name = entry
					.file_name()
					.and_then(|name| name.to_str())
					.ok_or_else(|| {
						bevyhow!("entry `{entry}` has no file name")
					})?
					.to_string();
				Ok((dir, entry_name))
			}
		},
	}
}

/// The entry path relative to `root` as a utf8 string, or `None` if `entry` is not
/// under `root` (or the relative path is not utf8).
fn entry_name_under(entry: &AbsPathBuf, root: &AbsPathBuf) -> Option<String> {
	entry.strip_prefix(root).ok()?.to_str().map(str::to_string)
}

/// The default store root when no `--root` is given: the workspace root, so an entry
/// reaches workspace paths (eg `assets/`) with no flag. `None` on wasm (no workspace)
/// or if detection fails, so the caller falls back to the entry's own directory.
#[cfg(not(target_arch = "wasm32"))]
fn default_entry_root() -> Option<AbsPathBuf> {
	AbsPathBuf::new(fs_ext::workspace_root()).ok()
}
#[cfg(target_arch = "wasm32")]
fn default_entry_root() -> Option<AbsPathBuf> { None }

/// The native entry resolution: a remote store, an explicit `--main`, or a
/// filesystem ancestor walk; see [`resolve_entry`].
#[cfg(not(target_arch = "wasm32"))]
async fn resolve_entry_native(args: &CliArgs) -> Result<ResolvedEntry> {
	// remote: load the whole entry from the store the deploy injected; there is no
	// local directory to watch. The concept is feature-agnostic, only the backend is
	// gated, so an unmatched remote-access request errors with guidance.
	if remote_access() {
		#[cfg(feature = "aws_sdk")]
		{
			let (store, entry_name) = remote_entry_store()?;
			return Ok(ResolvedEntry {
				store,
				entry_name,
				watch_dir: None,
			});
		}
		#[cfg(not(feature = "aws_sdk"))]
		bevybail!(
			"BEET_SERVICE_ACCESS=remote but no remote store backend is compiled in \
			(enable the `aws_sdk` feature)"
		);
	}

	// the wasm runner forwards the *module's* flags on this same argv, so a
	// `beet run-wasm <module> --main=<wasm-entry> --store=fs ...` invocation
	// carries a `--main`/`--store` meant for the wasm module, not this native
	// runner. When acting as the runner (first positional `run-wasm`), ignore them
	// and discover the workspace command entry; the `<RunWasm/>` route forwards the
	// flags on to the module via `Deno.args`.
	let is_wasm_runner =
		args.path.first().map(SmolStr::as_str) == Some("run-wasm");

	// local: the binary's own `--main` overrides discovery, otherwise discovery walks
	// for the dir + entry name. Either way the `--store` arg picks the backend.
	match args
		.params
		.get("main")
		.filter(|_| !is_wasm_runner)
		.map(|path| AbsPathBuf::new(path.as_str()))
		.transpose()?
	{
		Some(entry) => {
			let (dir, entry_name) = entry_root_and_name(&args.params, &entry)?;
			Ok(ResolvedEntry {
				store: resolve_store(&args.params, dir.clone())?,
				entry_name,
				watch_dir: Some(dir),
			})
		}
		None => discover_entry().await,
	}
}

/// Whether the runtime should access services remotely (the deployed task), read
/// from `BEET_SERVICE_ACCESS`. Feature-agnostic: a remote backend (eg `aws_sdk`'s
/// S3) is gated separately, since there are non-S3 reasons to access remotely.
#[cfg(not(target_arch = "wasm32"))]
fn remote_access() -> bool {
	env_ext::var("BEET_SERVICE_ACCESS")
		.map(|value| value.eq_ignore_ascii_case("remote"))
		.unwrap_or(false)
}

/// A [`BlobStore`] backed by the deploy's S3 entry bucket (`BEET_SITE_BUCKET`); the
/// entry document is `BEET_SITE_ENTRY` (default `main.bsx`) at the bucket root. It is
/// deploy config, not discovery, since a remote task has no local `main.bsx` to walk
/// to.
///
/// An explicit `BEET_S3_ENDPOINT` (eg `https://<account>.r2.cloudflarestorage.com`)
/// switches the store onto an S3-compatible service such as Cloudflare R2: the
/// region becomes `auto`, path-style addressing is used, and the same `AWS_*`
/// keys carry the R2 credentials. Unset, it reads AWS S3 in `AWS_REGION`. So one
/// container binary serves identically on Fargate (S3) and Cloudflare (R2).
#[cfg(feature = "aws_sdk")]
fn remote_entry_store() -> Result<(BlobStore, String)> {
	let bucket = env_ext::var("BEET_SITE_BUCKET").map_err(|_| {
		bevyhow!("BEET_SERVICE_ACCESS=remote but BEET_SITE_BUCKET is unset")
	})?;
	let store = match env_ext::var("BEET_S3_ENDPOINT") {
		Ok(endpoint) => {
			info!("loading entry from r2/s3 bucket `{bucket}` ({endpoint})");
			S3Store::new(bucket, "auto").with_endpoint(endpoint)
		}
		Err(_) => {
			let region = env_ext::var("AWS_REGION")
				.unwrap_or_else(|_| "us-west-2".to_string());
			info!("loading entry from s3 bucket `{bucket}` ({region})");
			S3Store::new(bucket, region)
		}
	};
	let entry_name = env_ext::var("BEET_SITE_ENTRY")
		.unwrap_or_else(|_| "main.bsx".to_string());
	Ok((BlobStore::new(store), entry_name))
}

/// Walk the cwd and its ancestors for the first [`ENTRY_NAMES`] match, resolving
/// through an `fs` [`BlobStore`] at each candidate dir (consistent with the store
/// API and async, rather than a raw `fs_ext` probe). Discovery is the only native
/// place a filesystem walk makes sense; it returns the matched store, entry name,
/// and dir, erroring with guidance when none is found.
#[cfg(not(target_arch = "wasm32"))]
async fn discover_entry() -> Result<ResolvedEntry> {
	let start = AbsPathBuf::new(".")?;
	let mut dir = Some(start.clone());
	while let Some(current) = dir {
		let store = BlobStore::new(FsStore::new(current.clone()));
		for name in ENTRY_NAMES {
			if store.exists(&SmolPath::from(*name)).await? {
				return Ok(ResolvedEntry {
					store,
					entry_name: name.to_string(),
					watch_dir: Some(current),
				});
			}
		}
		dir = current.parent();
	}
	bevybail!(
		"no entry document found: looked for {ENTRY_NAMES:?} in `{start}` and its \
		ancestors. Create a `main.bsx` or pass `--main=<path>`."
	)
}
