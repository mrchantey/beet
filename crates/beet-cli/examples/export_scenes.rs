//! Generates the `utils-cli` scene: the `beet` CLI's optional utility commands
//! (run-wasm, build-wasm, export-pdf, s3-sync, and under `qrcode` qrcode)
//! bundled as a loadable scene.
//!
//! The `beet` binary itself only carries scene management; running this example
//! writes `target/scenes/utils-cli.json`, which `beet load`s to add the utility
//! routes.
//!
//! ```sh
//! cargo run -p beet-cli --example export_scenes   # writes target/scenes/utils-cli.json
//! beet load target/scenes/utils-cli.json
//! beet run-wasm --help
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
			// register the utility command types so they serialize into the scene.
			CliCommandsPlugin,
		))
		// spawn the tagged scene root, then [`export_scenes`] writes it to disk.
		.add_systems(Startup, (spawn_scene, export_scenes).chain())
		.run_once()
		.unwrap_or(AppExit::Success)
}

/// Spawn the scene root — a bare [`Router`] carrying the utility commands —
/// tagged with the [`ExportScene`] route and its [`ExportPath`]. The export
/// markers are denied by the saver, so they stay out of the output. A plain
/// [`Router`] (not [`default_router`]) avoids re-adding the host's app routes,
/// which would clash when the scene loads under another router. [`export_scenes`]
/// then writes it to `target/scenes/utils-cli.json`.
fn spawn_scene(mut commands: Commands) {
	commands.spawn((
		ExportScene,
		ExportPath("target/scenes/utils-cli.json".into()),
		Router,
		children![
			RunWasm,
			BuildWasm,
			ExportPdf,
			SyncS3,
			#[cfg(feature = "qrcode")]
			QrCode,
		],
	));
}
