//! The `beet` binary: discover an entry, supply the process request, load it,
//! let the loaded tree run itself, and exit unless something kept it alive.
//!
//! beet is unopinionated like a game engine: it links a library of capabilities
//! (registered reflect types) but ships zero behaviour. It discovers `main.bsx`
//! (or `main.json`/`main.ron`) by walking the cwd and its ancestors, consumes
//! only its own `--main` flag, and builds the entry through the unified
//! [`TemplateLoader`], then lets the load-lifecycle verbs (`EvalOnLoad`,
//! `ServeOnLoad`) fire on the build's `LoadTemplate`, each re-parsing argv for its
//! own input. It exits once the [`KeepAlive`] refcount falls to zero; a long-running
//! server holds a ref to persist the process.
use beet::prelude::*;
use beet_cli::prelude::*;

/// Entry-document file names discovery looks for, in priority order, walking the
/// cwd and its ancestors (a `Cargo.toml`-style walk).
const ENTRY_NAMES: &[&str] = &["main.bsx", "main.json", "main.ron"];

fn main() -> AppExit {
	// load any local `.env` (eg `BEET_REMOTE_URL`) before the app starts.
	env_ext::load_dotenv();

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LogPlugin::new(Level::DEBUG),
		ClientAppPlugin,
		// dev-command capabilities stay linked as registered types, inert until a
		// `main.bsx` names them; the binary spawns no host, route, or command.
		CliCommandsPlugin,
		// the device-push capabilities, likewise inert: the host push commands
		// (`<SceneLoad/>`, ...) and the device-receive meta-routes (`<SceneServer/>`).
		SceneManagementPlugin,
		SceneServerPlugin,
		// the rule set a presented/served site renders with.
		material::MaterialStylePlugin::default(),
		// the stack-of-cards machinery, dormant unless a `CardDeck` is present.
		CardStackPlugin,
	))
	.init_plugin::<AsyncPlugin>();
	// the live terminal target the `TuiServer` boots into. `init_plugin` is
	// idempotent, so `NavigatorPlugin` (already added by `ClientAppPlugin`) is not
	// added twice.
	#[cfg(feature = "tui")]
	app.init_plugin::<CharcellTuiPlugin>()
		.init_plugin::<NavigatorPlugin>()
		.init_plugin::<LivePagePlugin>();
	// the multi-tenant SSH-TUI server's per-connection behavior, so a served site
	// declaring `<.. SshTuiServer>` serves each ssh session its own terminal.
	#[cfg(feature = "ssh")]
	app.init_plugin::<SshTuiPlugin>();

	app.add_systems(Startup, load_entry)
		.add_systems(
			Last,
			exit_when_unclaimed.run_if(resource_exists_and_changed::<KeepAlive>),
		)
		.run()
}

/// `Startup`: resolve the entry and build it through the unified [`TemplateLoader`].
/// The build fires `LoadTemplate` on the root, running the tree's verbs. A load-scope
/// [`KeepAlive`] ref is held across the build so the process cannot exit mid-load;
/// any verb that boots a long-running server (or queues the cli exchange) takes its
/// own ref before this one drops. Any failure logs and exits with an error rather
/// than panicking. Run here (not before the app) so the message goes through the
/// initialized logger.
fn load_entry(world: &mut World) {
	// hold the load-scope ref, then build. The verbs' boots cascade through deferred
	// commands, so flush before dropping the ref to ensure any lasting claim is held.
	world.resource_mut::<KeepAlive>().acquire();
	match try_load_entry(world) {
		Ok(()) => {
			world.flush();
			world.resource_mut::<KeepAlive>().release();
		}
		Err(err) => {
			// a failed build keeps its ref (the exit system emits no success) and
			// exits with an error code instead.
			error!("{err}");
			world.write_message(AppExit::error());
		}
	}
}

fn try_load_entry(world: &mut World) -> Result {
	// the binary's own `--main` overrides discovery; the loaded tree re-parses argv
	// itself (the load verbs call `CliArgs::parse_env`), so the binary consumes only
	// its own `--main` here.
	let mut args = CliArgs::parse_env();
	let entry = match args
		.params
		.remove("main")
		.and_then(|values| values.into_iter().next())
	{
		Some(path) => AbsPathBuf::new(path.as_str())?,
		None => discover_entry()?,
	};

	// the entry's directory is its project root: register a sibling `templates/`
	// (so `<path::to::X>` / `<Styles/>` templates resolve) and set the `SiteRoot`
	// (which `<RoutesDir/>` resolves against), so a no-code site entry's templates
	// and routes load. Both are no-ops for a single-file entry that uses neither.
	if let Some(dir) = entry.parent() {
		let templates = dir.join("templates");
		if fs_ext::exists(&templates)? {
			world.register_bsx_templates(templates)?;
		}
		world.insert_resource(SiteRoot(dir));
	}

	let media = fs_ext::read_media(&entry)?;
	TemplateLoader::new(world)
		.load(&media)
		.map_err(|err| bevyhow!("failed to load entry `{entry}`: {err}"))?;
	Ok(())
}

/// Walk the cwd and its ancestors for the first [`ENTRY_NAMES`] match, erroring
/// with guidance when none is found.
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

/// `Last`: exit once nothing is keeping the process alive. [`KeepAlive`] refcounts
/// the live claims (the load scope, each running server or cli exchange); when the
/// last ref drops the count reaches zero and the process exits cleanly. A failed
/// command writes its own `AppExit::Error` first to carry its exit code. Gated on a
/// `KeepAlive` change, so there is no per-frame polling.
fn exit_when_unclaimed(
	keep_alive: Res<KeepAlive>,
	mut exit: MessageWriter<AppExit>,
) {
	if keep_alive.count() == 0 {
		exit.write(AppExit::Success);
	}
}
