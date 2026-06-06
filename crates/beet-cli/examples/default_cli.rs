//! Generates the default `beet` CLI as a `beet.json` scene.
//!
//! The `beet` binary is a bare scene runner; the real CLI lives in a scene. This
//! example is itself a [`CliServer`] whose only route is [`ExportScene`]; running
//! it serializes [`ExportScene`]'s descendant scene — the build/run/export
//! commands plus the load/clear/reset/dump/run scene-control commands — to a
//! `beet.json` via world serde, which `beet` then loads and runs.
//!
//! ```sh
//! cargo run -p beet-cli --example default_cli -- --output ./beet.json
//! beet --help
//! beet load scenes/led-script.json   # with BEET_REMOTE_URL set, drives a device
//! ```
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	env_ext::load_dotenv();
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ClientAppPlugin,
			CliCommandsPlugin,
			SceneCommandsPlugin,
		))
		.add_systems(Startup, spawn_export_server)
		.run()
}

/// Spawn the export server: a [`CliServer`] whose [`ExportScene`] route carries
/// the default CLI scene as its children. Running `export` serializes that scene
/// (each child a root) to the `--output` path.
fn spawn_export_server(world: &mut World) {
	world.spawn((CliServer, default_router(), children![(
		ExportScene,
		children![
			RunWasm,
			BuildWasm,
			ExportPdf,
			#[cfg(feature = "qrcode")]
			QrCode,
			SceneLoad,
			SceneClear,
			SceneReset,
			SceneDump,
			SceneRun,
		],
	)]));
}
