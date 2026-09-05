//! The `Startup` entry loader: resolve a repo store and an entry document name,
//! build the entry into a root that carries the store, and let the loaded tree
//! run itself.
//!
//! Added by [`LaunchPlugin`], which is the whole of what makes a binary a beet
//! runtime beyond its chosen capability plugins.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Adds the [`Startup`] entry loader, ie the half of a beet binary that is not
/// its capabilities.
///
/// A binary is otherwise unopinionated: it links a library of capabilities
/// (registered reflect types) and ships zero behaviour, and the entry document
/// decides what runs. This plugin is what finds and builds that document.
///
/// The binary spawns its own [`CrateRegistration`] before `run`, marked
/// [`with_skip_prefix`](CrateRegistration::with_skip_prefix): a `<CrateCheck/>`
/// naming an unprefixed feature resolves against it, and only the binary knows
/// which cargo features it was compiled with. A binary that spawns none fails
/// any unprefixed check with a message saying so.
#[derive(Debug, Default, Clone, Copy)]
pub struct LaunchPlugin;

impl Plugin for LaunchPlugin {
	fn build(&self, app: &mut App) { app.add_systems(Startup, load_entry); }
}

/// Positional commands that run ANOTHER program, so the `--main` and `--repo`
/// on this process's argv belong to that program and are forwarded untouched
/// rather than read as this launch's own.
///
/// Absent by default: a binary whose commands all run in-process has nothing to
/// forward. `beet run-wasm <module>` is the one beet ships, and the crate that
/// owns that command is the one that inserts this.
#[derive(Debug, Default, Clone, Resource)]
pub struct ArgvPassthrough(Vec<SmolStr>);

impl ArgvPassthrough {
	/// Declare the commands whose argv belongs to a child program.
	pub fn new(commands: impl IntoIterator<Item = impl Into<SmolStr>>) -> Self {
		Self(commands.into_iter().map(Into::into).collect())
	}

	/// Whether `args` names one of them, ie whether this launch's `--main` and
	/// `--repo` are somebody else's.
	pub fn forwards(&self, args: &CliArgs) -> bool {
		args.path
			.first()
			.is_some_and(|command| self.0.contains(command))
	}
}

/// `Startup`: resolve the repo store + name and build the entry, all on the async
/// runtime so discovery (a store walk), template registration, and every store read
/// (`templates/`, the entry document, `<RoutesDir>`/`<Template src>`) go through the
/// one [`BlobStore`] without ever blocking the runtime (which is single-threaded on
/// wasm). The app loop drives the task; its build fires `Ready` on the root,
/// where the `CallOnReady` verb fans the process request out to the entry's servers.
/// The app then stays alive until something writes `AppExit`, so nothing is held by
/// hand here. A failed resolve/build logs and exits with an error rather than
/// panicking. Target-agnostic: every runtime builds the same way, differing only
/// in how [`resolve_entry`] finds the store.
///
/// The binary's own [`CrateRegistration`] is NOT spawned here: it names the
/// binary's cargo features, which only the binary knows, so it spawns its own
/// before `run` (see [`LaunchPlugin`]).
fn load_entry(world: &mut World) {
	// the binary consumes only its own args here; the loaded tree re-parses argv.
	let args = CliArgs::parse_env();
	// whether those args belong to a CHILD program this process is only hosting,
	// declared by whichever crate owns such a command.
	let forwards_argv = world
		.get_resource::<ArgvPassthrough>()
		.is_some_and(|passthrough| passthrough.forwards(&args));
	// the process config, parsed strictly once: entry resolution and the build
	// both read it, and a malformed knob fails the launch rather than warning.
	let config = match BootstrapConfig::from_env() {
		Ok(config) => config,
		Err(err) => {
			error!("{err}");
			world.write_message(AppExit::error());
			return;
		}
	};
	if let Some(check) = features_self_check(&args, &config, forwards_argv) {
		world.spawn(check);
	}
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
		let resolved = match resolve_entry(&config, forwards_argv).await {
			Ok(resolved) => resolved,
			Err(err) => {
				error!("{err}");
				world.write_message(AppExit::error()).await;
				return;
			}
		};
		if let Err(err) = build_entry(&world, &config, resolved, formats).await
		{
			error!("{err}");
			world.write_message(AppExit::error()).await;
		}
	});
}

/// Build the browser entry: read the program from the DOM via
/// [`MainBsx::read_dom_program`] and build it onto a storeless root (see
/// [`entry_build::build_from_bsx`]). The wasm `Browser` branch of
/// [`load_entry`]; the program's own `CallOnReady` verb then drives it.
#[cfg(target_arch = "wasm32")]
async fn browser_entry(world: &AsyncWorld, formats: TemplateFormats) -> Result {
	let bsx = MainBsx::read_dom_program().await?;
	world
		.with(move |world: &mut World| {
			entry_build::build_from_bsx(world, formats, "main.bsx", bsx)
		})
		.await?;
	Ok(())
}

/// Build the resolved entry on the async runtime: register the entry's `templates/`
/// and read the entry document through the store (awaited, not blocked), then build
/// it into a root carrying the store so `<RoutesDir>` and `<Template src>` resolve
/// the store by ancestry. The entry runs itself: its own `CallOnReady` acts on its
/// `Ready` and boots the servers. Target-agnostic; the `--watch` live-reload path
/// is native-only.
async fn build_entry(
	world: &AsyncWorld,
	config: &BootstrapConfig,
	resolved: ResolvedEntry,
	formats: TemplateFormats,
) -> Result {
	let ResolvedEntry {
		repo_store,
		entry_name,
		prescan,
		#[cfg(not(target_arch = "wasm32"))]
		watch_dir,
	} = resolved;
	// native `--watch` (local dev): install the live-reload driver and build through
	// the shared rebuild path, so the initial build and a structural rebuild are
	// identical and the entry root is a `BeetSceneRoot` the reload can tear down.
	// Opt-in, so a running presentation never reloads underfoot; a deployed (remote)
	// entry has no local dir to watch, the wasm runner has no fs watcher, and a
	// binary without `client_io` has no reload channel to serve.
	#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
	if watch_dir.is_some() && config.watch {
		return build_watched_entry(world, repo_store, entry_name, formats)
			.await;
	}
	// otherwise the plain one-shot build. The binary stays unopinionated: it
	// simply loads, and the entry's own markup decides how it runs by carrying
	// a `CallOnReady` or not.
	#[cfg(any(target_arch = "wasm32", not(feature = "client_io")))]
	let _ = config;
	#[cfg(all(not(feature = "client_io"), not(target_arch = "wasm32")))]
	let _ = watch_dir;
	let sources = entry_build::read_sources(
		&repo_store,
		formats,
		entry_name.clone(),
		prescan,
	)
	.await?;
	world
		.with(move |world: &mut World| -> Result {
			entry_build::build_root(world, repo_store, sources, RepoStore)?;
			Ok(())
		})
		.await
}

/// The `--watch` entry build (native-only): install the live-reload driver
/// ([`EntryReloader`]), then do the first build through the same
/// [`entry_build::rebuild_watched`] path a structural change re-runs (which also
/// recomputes the structural source set — the entry document and its transitive
/// `<Template src>` includes — per build).
///
/// So editing the entry document or an included `<Template src>` tears the old
/// scene down and rebuilds it with no leaked entities (servers rebind, sockets
/// reconnect), while a markdown/template edit keeps the light content re-fire.
#[cfg(all(feature = "client_io", not(target_arch = "wasm32")))]
async fn build_watched_entry(
	world: &AsyncWorld,
	repo_store: BlobStore,
	entry_name: String,
	formats: TemplateFormats,
) -> Result {
	// the driver's rebuild callback, re-cloning the store/name/formats per build
	// (it is an `Fn`, re-run on every structural change). The structural source
	// set starts empty; the first build below populates it.
	let rebuild = {
		let repo_store = repo_store.clone();
		let entry_name = entry_name.clone();
		let formats = formats.clone();
		move |world: AsyncWorld| -> LocalBoxedFuture<'static, Result> {
			let (repo_store, entry_name, formats) =
				(repo_store.clone(), entry_name.clone(), formats.clone());
			Box::pin(async move {
				entry_build::rebuild_watched(
					&world, repo_store, entry_name, formats,
				)
				.await
			})
		}
	};
	world
		.with(move |world: &mut World| {
			world.insert_resource(EntryReloader::new(default(), rebuild));
		})
		.await;
	// the first build: a no-op teardown, then the fresh `BeetSceneRoot`.
	entry_build::rebuild_watched(world, repo_store, entry_name, formats).await
}

/// Resolve the entry [`BlobStore`], the entry document name within it, and the
/// local directory to watch for dev live reload (`None` when the store has no
/// local root, ie a self-rooted store).
///
/// Resolution order:
/// 1. a self-rooted `--repo` (`s3://<bucket>`, `local-storage`, `indexed-db`):
///    the store roots itself, so `--main` names the entry document *within* it,
///    defaulting to an [`entry_build::ENTRY_NAMES`] probe. A deployed task passes
///    `--repo=s3://<bucket>` (deploy config as args, not env).
/// 2. `--main=<path>`: the entry file itself (a recognized extension) or a
///    directory probed for [`entry_build::ENTRY_NAMES`]; see [`entry_build::resolve_main`].
/// 3. otherwise: discovery walks the cwd and its ancestors through an `fs` store
///    for the first [`entry_build::ENTRY_NAMES`] match.
///
/// Every path then resolves through [`entry_build::resolve_in_repo_store`], so an
/// entry's `<RepoRoot src>` declaration rebases any store kind uniformly (an
/// fs store re-roots, a self-rooted store takes a key-prefix view or fails
/// loudly on a mis-publish). The config's [`StoreUri`] selects the backend
/// (default `fs`).
///
/// Target-agnostic: wasm runs the same walk wherever the runtime has a
/// filesystem (deno/node through the runner's fs globals); a fs-less runtime
/// errors with guidance (the browser never reaches here, reading its DOM program
/// instead).
async fn resolve_entry(
	config: &BootstrapConfig,
	forwards_argv: bool,
) -> Result<ResolvedEntry> {
	// the wasm runner forwards the *module's* flags on this same argv, so a
	// `beet run-wasm <module> --main=<wasm-entry> --repo=fs ...` invocation
	// carries a `--main`/`--repo` meant for the wasm module, not this native
	// runner. When acting as the runner (first positional `run-wasm`), drop them
	// and discover the workspace command entry; the `<RunWasm/>` route forwards the
	// module's own config on via `ChildProcess::with_bootstrap`.
	let repo_uri = (!forwards_argv).then(|| config.repo.as_ref()).flatten();
	let main = (!forwards_argv).then(|| config.main.as_ref()).flatten();

	// a self-rooted store: no local dir and no ancestor walk, so `--main` is a
	// key within the store, defaulting to the entry-name probe.
	if repo_uri.is_some_and(StoreUri::is_self_rooted) {
		let repo_store =
			entry_build::resolve_repo_store(repo_uri, AbsPathBuf::new(".")?)?;
		let entry_name = match main {
			Some(main) => main.to_string(),
			None => entry_build::probe_entry_names(&repo_store)
				.await?
				.ok_or_else(|| {
					bevyhow!(
						"no entry document found in the `--repo` backend: looked \
					for {:?}. Seed one, or pass `--main=<name>`.",
						entry_build::ENTRY_NAMES
					)
				})?,
		};
		return entry_build::resolve_in_repo_store(repo_store, entry_name)
			.await;
	}

	// dir-rooted: an explicit `--main`, else the ancestor walk. On wasm the `fs`
	// store reads through the runner's fs globals, so a fs-less runtime cannot
	// resolve a dir-rooted entry at all.
	#[cfg(target_arch = "wasm32")]
	if !js_runtime::environment().has_fs() {
		bevybail!(
			"this runtime has no filesystem: pass a self-rooted `--repo` \
			(s3://<bucket>, local-storage, indexed-db)"
		);
	}
	match main {
		Some(main) => entry_build::resolve_main(repo_uri, main.as_str()).await,
		None => discover_entry(repo_uri).await,
	}
}

/// The `--features` flag as a [`CrateCheck`]: verify this binary was compiled
/// with the named cargo features, failing with the full missing list rather
/// than degrading into unresolved tags. Applies when running an entry (an
/// explicit `--main`, or no positional command); a command dispatch (eg `beet
/// build-wasm --features=..`) owns its own `--features` meaning, and the wasm
/// runner forwards the module's flags untouched.
fn features_self_check(
	args: &CliArgs,
	config: &BootstrapConfig,
	forwards_argv: bool,
) -> Option<CrateCheck> {
	let runs_entry = config.main.is_some() || args.path.is_empty();
	if forwards_argv || !runs_entry || config.features.is_empty() {
		return None;
	}
	Some(CrateCheck::features(config.features.clone()))
}

/// Walk the cwd and its ancestors for the first [`entry_build::ENTRY_NAMES`] match, resolving
/// through an `fs` [`BlobStore`] at each candidate dir (consistent with the store
/// API and async, rather than a raw `fs_ext` probe). Discovery is the only place
/// a filesystem walk makes sense; the matched entry may still rebase its own
/// root ([`entry_build::resolve_in_repo_store`]), and no match errors with guidance.
async fn discover_entry(repo_uri: Option<&StoreUri>) -> Result<ResolvedEntry> {
	let start = AbsPathBuf::new(".")?;
	let mut dir = Some(start.clone());
	while let Some(current) = dir {
		let repo_store = BlobStore::new(FsStore::new(current.clone()));
		if let Some(entry_name) =
			entry_build::probe_entry_names(&repo_store).await?
		{
			let repo_store =
				entry_build::resolve_repo_store(repo_uri, current)?;
			return entry_build::resolve_in_repo_store(repo_store, entry_name)
				.await;
		}
		dir = current.parent();
	}
	bevybail!(
		"no entry document found: looked for {:?} in `{start}` and its \
		ancestors. Create a `main.bsx` or pass `--main=<path>`.",
		entry_build::ENTRY_NAMES
	)
}
