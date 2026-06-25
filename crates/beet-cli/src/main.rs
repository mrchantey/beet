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
// the wasm build is a Cloudflare Worker `cdylib` (see `lib.rs`'s `#[event(fetch)]`
// entry, built with `--lib`); the bin target keeps an empty `main` so it still
// links if ever built for wasm.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use beet::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use beet_cli::prelude::*;

/// Entry-document file names discovery looks for, in priority order, walking the
/// cwd and its ancestors (a `Cargo.toml`-style walk).
#[cfg(not(target_arch = "wasm32"))]
const ENTRY_NAMES: &[&str] = &["main.bsx", "main.json", "main.ron"];

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
			// the native-only dev-command capabilities, linked as registered types and
			// inert until a `main.bsx` names them.
			CliCommandsPlugin,
			// the example capabilities, so a `main.bsx` declaring example tags resolves:
			// the agent-thread runtime + chat UI (`thread`, eg `examples/thread/chat.bsx`)
			// and the windowed spatial/animation scene types + templates (`render`, eg
			// `examples/spatial/seek_3d.bsx`). The group self-selects by sub-feature.
			#[cfg(any(feature = "thread", feature = "render"))]
			beet_examples::prelude::BeetExamplePlugins,
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

/// `Startup`: resolve the site store + entry name (env/discovery only, no I/O),
/// then build the entry on the async runtime so template registration and every
/// store read (`templates/`, the entry document, `<RoutesDir>`/`<Template src>`)
/// go through the one [`BlobStore`] without ever blocking the runtime (which is
/// single-threaded on wasm). The app loop drives the task; its build fires
/// `LoadTemplate` on the root, where the `BootOnLoad` verb fans the process request
/// out to the entry's servers. The app then stays alive until something writes
/// `AppExit`, so nothing is held by hand here. A failed resolve/build logs and
/// exits with an error rather than panicking.
#[cfg(not(target_arch = "wasm32"))]
fn load_entry(world: &mut World) {
	let resolved = match resolve_site_store() {
		Ok(resolved) => resolved,
		Err(err) => {
			error!("{err}");
			world.write_message(AppExit::error());
			return;
		}
	};
	// the recognized template formats (`.bsx`, `.js`), read once here so the async
	// build can both filter the `templates/` read and lower each source by format.
	let formats = world.get_resource_or_init::<TemplateFormats>().clone();
	world.run_async_local(async move |world: AsyncWorld| {
		if let Err(err) = build_entry(&world, resolved, formats).await {
			error!("{err}");
			world.write_message(AppExit::error()).await;
		}
	});
}

/// A resolved site entry: its store, the entry document name within it, and the
/// local dir to watch for live reload (`None` for a remote site).
#[cfg(not(target_arch = "wasm32"))]
struct ResolvedSite {
	store: BlobStore,
	entry_name: String,
	watch_dir: Option<AbsPathBuf>,
}

/// Build the resolved entry on the async runtime: register the site `templates/`
/// and read the entry document through the store (awaited, not blocked), then build
/// it into a root carrying the site store so `<RoutesDir>` and `<Template src>`
/// resolve the store by ancestry. The build fires `LoadTemplate`, where `BootOnLoad`
/// boots the servers.
#[cfg(not(target_arch = "wasm32"))]
async fn build_entry(
	world: &AsyncWorld,
	resolved: ResolvedSite,
	formats: TemplateFormats,
) -> Result {
	let ResolvedSite {
		store,
		entry_name,
		watch_dir,
	} = resolved;
	let sources = read_site_sources(&store, formats, entry_name).await?;
	world
		.with(move |world: &mut World| -> Result {
			// the binary stays unopinionated: it spawns the entry root with no load
			// verb of its own, so the entry's own markup declares how it loads. A
			// server entry spreads `BootOnLoad` beside its servers, a script entry
			// spreads `ExchangeOnLoad`, a render scene `RunOnLoad`, and a self-booting
			// verb (eg a thread's `{CreateThread}`) `#[require]`s `BootOnLoad` itself.
			let root = build_site_root(world, store, sources, ())?;
			// `--watch` (local dev): mark the root for live reload. Its `FsStore`'s
			// watcher already emits `BlobEvent`s, so editing a template/slide/style
			// hot-reloads connected browsers, the deck's `<LiveReloadScript/>` widget
			// turning the broadcast into a reload. Opt-in, so a running presentation
			// never reloads underfoot; a deployed (S3) site has no local dir to watch.
			if watch_dir.is_some()
				&& CliArgs::parse_env().params.contains_key("watch")
			{
				world.entity_mut(root).insert(LiveReload::new());
				world.flush();
			}
			Ok(())
		})
		.await
}

/// Resolve the site [`BlobStore`], the entry document name within it, and the
/// local directory to watch for dev live reload (`None` when there is no local
/// dir, ie a remote/S3 site).
///
/// A deployed task (`BEET_SERVICE_ACCESS=remote`) loads the site from its S3
/// bucket; otherwise discovery walks the filesystem for a local `main.bsx`.
#[cfg(not(target_arch = "wasm32"))]
fn resolve_site_store() -> Result<ResolvedSite> {
	// remote: pull the whole site from the S3 bucket the deploy injected; there is
	// no local directory to watch.
	#[cfg(feature = "aws_sdk")]
	if remote_access() {
		let (store, entry_name) = remote_site_store()?;
		return Ok(ResolvedSite {
			store,
			entry_name,
			watch_dir: None,
		});
	}

	// local: the binary's own `--main` overrides discovery; the loaded tree
	// re-parses argv itself, so the binary consumes only its own `--main` here.
	let mut args = CliArgs::parse_env();
	let entry = match args
		.params
		.remove("main")
		.and_then(|values| values.into_iter().next())
	{
		Some(path) => AbsPathBuf::new(path.as_str())?,
		None => discover_entry()?,
	};
	let dir = entry
		.parent()
		.ok_or_else(|| bevyhow!("entry `{entry}` has no parent directory"))?;
	let entry_name = entry
		.file_name()
		.and_then(|name| name.to_str())
		.ok_or_else(|| bevyhow!("entry `{entry}` has no file name"))?
		.to_string();
	Ok(ResolvedSite {
		store: BlobStore::new(FsStore::new(dir.clone())),
		entry_name,
		watch_dir: Some(dir),
	})
}

/// Whether the runtime should access services remotely (the deployed task), read
/// from `BEET_SERVICE_ACCESS`.
#[cfg(feature = "aws_sdk")]
fn remote_access() -> bool {
	env_ext::var("BEET_SERVICE_ACCESS")
		.map(|value| value.eq_ignore_ascii_case("remote"))
		.unwrap_or(false)
}

/// A [`BlobStore`] backed by the deploy's S3 site bucket (`BEET_SITE_BUCKET`); the
/// entry document is `BEET_SITE_ENTRY` (default `server.bsx`) at the bucket root
/// (the lean serve entry the container loads directly, skipping the dev `main.bsx`
/// include indirection). It is deploy config, not discovery, since a remote task
/// has no local `main.bsx` to walk to.
///
/// An explicit `BEET_S3_ENDPOINT` (eg `https://<account>.r2.cloudflarestorage.com`)
/// switches the store onto an S3-compatible service such as Cloudflare R2: the
/// region becomes `auto`, path-style addressing is used, and the same `AWS_*`
/// keys carry the R2 credentials. Unset, it reads AWS S3 in `AWS_REGION`. So one
/// container binary serves identically on Fargate (S3) and Cloudflare (R2).
#[cfg(feature = "aws_sdk")]
fn remote_site_store() -> Result<(BlobStore, String)> {
	let bucket = env_ext::var("BEET_SITE_BUCKET").map_err(|_| {
		bevyhow!("BEET_SERVICE_ACCESS=remote but BEET_SITE_BUCKET is unset")
	})?;
	let store = match env_ext::var("BEET_S3_ENDPOINT") {
		Ok(endpoint) => {
			info!("loading site from r2/s3 bucket `{bucket}` ({endpoint})");
			S3Store::new(bucket, "auto").with_endpoint(endpoint)
		}
		Err(_) => {
			let region = env_ext::var("AWS_REGION")
				.unwrap_or_else(|_| "us-west-2".to_string());
			info!("loading site from s3 bucket `{bucket}` ({region})");
			S3Store::new(bucket, region)
		}
	};
	let entry_name = env_ext::var("BEET_SITE_ENTRY")
		.unwrap_or_else(|_| "server.bsx".to_string());
	Ok((BlobStore::new(store), entry_name))
}

/// Walk the cwd and its ancestors for the first [`ENTRY_NAMES`] match, erroring
/// with guidance when none is found.
#[cfg(not(target_arch = "wasm32"))]
fn discover_entry() -> Result<AbsPathBuf> {
	let start = AbsPathBuf::new(".")?;
	let mut dir = Some(start.clone());
	while let Some(current) = dir {
		for name in ENTRY_NAMES {
			let candidate = current.join(name);
			if fs_ext::exists(&candidate)? {
				return Ok(candidate);
			}
		}
		dir = current.parent();
	}
	bevybail!(
		"no entry document found: looked for {ENTRY_NAMES:?} in `{start}` and its \
		ancestors. Create a `main.bsx` or pass `--main=<path>`."
	)
}
