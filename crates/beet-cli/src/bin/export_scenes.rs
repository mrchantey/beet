//! Generates the `default-cli` scene: the `beet` CLI's default commands
//! (run, run-wasm, build-wasm, export-pdf, s3-sync, and under `qrcode` qrcode)
//! bundled as a loadable scene.
//!
//! The `beet` binary itself only carries scene management; running this bin
//! writes `target/scenes/default-cli.json`, which `beet load`s to add the
//! command routes.
//!
//! ```sh
//! cargo run -p beet-cli --bin export_scenes   # writes target/scenes/default-cli.json
//! beet load target/scenes/default-cli.json
//! beet run examples/bsx_site
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
			// register the command types so they serialize into the scene.
			CliCommandsPlugin,
		))
		// spawn the tagged scene root, then [`export_scenes`] writes it to disk.
		.add_systems(Startup, (spawn_scene, export_scenes).chain())
		.run_once()
		.unwrap_or(AppExit::Success)
}

/// Spawn the scene root — a bare [`Router`] carrying the default commands —
/// tagged with the [`ExportScene`] route and its [`ExportPath`]. The export
/// markers are denied by the saver, so they stay out of the output. A plain
/// [`Router`] (not [`default_router`]) avoids re-adding the host's app routes,
/// which would clash when the scene loads under another router. [`export_scenes`]
/// then writes it to `target/scenes/default-cli.json`.
fn spawn_scene(mut commands: Commands) {
	commands.spawn((
		ExportScene,
		ExportPath("target/scenes/default-cli.json".into()),
		Router,
		children![
			Run,
			RunWasm,
			BuildWasm,
			ExportPdf,
			SyncS3,
			#[cfg(feature = "qrcode")]
			QrCode,
		],
	));
}
