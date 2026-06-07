//! The `beet` command-line interface: a scene controller.
//!
//! `beet` is built around scene management: its only built-in commands are
//! `load`/`clear`/`reset`/`dump`/`run`. It carries *retained state* — on startup
//! it rehydrates the last scene from a local `.beet/scene.json`, so a
//! `beet load <file>` survives across invocations. Other capabilities
//! (`run-wasm`, `export-pdf`, …) ship as a loadable `utils-cli` scene generated
//! by the `export_scenes` example.
//!
//! [`SceneManagementPlugin`] wires the reactive cache + watcher machinery; the
//! CLI only has to spawn its host entity, which [`spawn_host`] does on startup
//! before the plugin's `rehydrate_scene_cache` loads any retained scene under it.
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	// load any local `.env` so eg `BEET_REMOTE_URL` is picked up by the scene
	// commands before the app starts.
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ClientAppPlugin,
			// register the loadable command types so a scene's markers can
			// reconstruct their behaviour: the `utils-cli` commands and the
			// built-in scene commands.
			CliCommandsPlugin,
			SceneManagementPlugin,
		))
		// spawn the host before the plugin rehydrates the retained scene under it.
		.add_systems(Startup, spawn_host.before(rehydrate_scene_cache))
		.run()
}

/// Spawn the CLI host: a single [`CliServer`] + [`default_router`] carrying the
/// built-in scene commands. Scenes load *under* it as route children, mirroring
/// the device's `HttpServer` host. The welcome page answers `/` until a loaded
/// scene supplies its own root route.
fn spawn_host(world: &mut World) {
	let host = world
		.spawn((CliServer, default_router(), children![
			SceneLoad,
			SceneClear,
			SceneReset,
			SceneDump,
			SceneRun,
		]))
		.id();
	world.entity_mut(host).with_child(scene_not_found_route());
}
