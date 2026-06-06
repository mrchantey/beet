//! Generates the default `beet` CLI as a `beet.json` scene.
//!
//! The `beet` binary is a bare scene runner; the real CLI (the
//! build-wasm/run-wasm/export-pdf/qrcode commands) lives in a scene. Running
//! this example serializes that scene to `./beet.json` via world serde, which
//! `beet` then loads and runs.
//!
//! ```sh
//! cargo run -p beet-cli --example default_cli
//! beet --help
//! ```
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ClientAppPlugin,
			CliCommandsPlugin,
		))
		.add_systems(Startup, export_default_cli)
		.run()
}

/// Spawn the default CLI bundle, serialize its entity tree to the
/// [`scene_output_path`] and exit. The transient [`CliServer`] short-circuits
/// its run hook once the entity is despawned, so spawning it just to serialize
/// is safe.
fn export_default_cli(world: &mut World) -> Result {
	let entity = world.spawn((CliServer::default(), default_router())).id();
	world.entity_mut(entity).with_children(|parent| {
		parent.spawn(RunWasm);
		parent.spawn(BuildWasm);
		parent.spawn(ExportPdf);
		#[cfg(feature = "qrcode")]
		parent.spawn(QrCode);
	});
	export_scene(world, entity)
}
