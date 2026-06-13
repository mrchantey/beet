//! Generates the `default-cli` scene: the `beet` CLI's default commands
//! (serve, run-wasm, build-wasm, export-pdf, s3-sync, and under `qrcode` qrcode)
//! bundled as a loadable scene.
//!
//! The `beet` binary itself only carries scene management; running this bin
//! writes `target/scenes/default-cli.json`, which `beet load`s to add the
//! command routes. It is a regular [`CliServer`]: an [`ExportScenes`] root route
//! whose children are the scenes to write, so running it with no args writes
//! them all.
//!
//! ```sh
//! cargo run -p beet-cli --bin export_scenes   # writes target/scenes/default-cli.json
//! beet load target/scenes/default-cli.json
//! beet serve examples/bsx_site
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
		.add_systems(Startup, spawn_host)
		.run()
}

/// Spawn the export host: a [`CliServer`] router whose root [`ExportScenes`]
/// route writes each of its children as a standalone scene. Each child is a
/// scene root carrying its [`ExportPath`]. The default-cli scene root is a plain
/// [`Router`] (not [`default_router`]), avoiding the host's app routes, which
/// would clash when the scene loads under another router.
fn spawn_host(mut commands: Commands) {
	commands.spawn((
		CliServer,
		Server::cli(),
		default_router(),
		children![(
			ExportScenes,
			children![(
				ExportPath("target/scenes/default-cli.json".into()),
				Router,
				children![
					Serve,
					RunWasm,
					BuildWasm,
					ExportPdf,
					SyncS3,
					#[cfg(feature = "qrcode")]
					QrCode,
				],
			)],
		)],
	));
}
