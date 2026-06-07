//! Generates the `utils-cli` scene: the `beet` CLI's optional utility commands
//! (run-wasm, build-wasm, export-pdf, and under `qrcode` qrcode) bundled as a
//! loadable scene.
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
		.add_systems(Startup, spawn_export_server)
		.run()
}

/// Spawn the export server: a [`CliServer`] whose [`ExportScene`] route carries
/// the utility commands as its children and an [`ExportPath`] for the output.
/// Running the example serializes that scene to `target/scenes/utils-cli.json`.
fn spawn_export_server(world: &mut World) {
	world.spawn((CliServer, default_router(), children![(
		ExportScene,
		ExportPath("target/scenes/utils-cli.json".into()),
		children![
			RunWasm,
			BuildWasm,
			ExportPdf,
			#[cfg(feature = "qrcode")]
			QrCode,
		],
	)]));
}
