//! Generates a remote-control `beet.json`: a scene whose routes fetch a
//! scene-server device (see `beet_router::scene_management`). Loaded by the
//! `beet` binary, it turns the CLI into a remote control —
//! `beet load <scene.json>` / `run` / `dump` / `clear` / `reset`.
//!
//! ```sh
//! cargo run -p beet-cli --example remote_loader -- --output ./beet.json
//! beet load scenes/led-script.json
//! ```
//!
//! The device URL is read at runtime from `SCENE_URL` (default
//! [`DEFAULT_DEVICE_URL`]).
use beet::prelude::*;
use beet_cli::prelude::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			ClientAppPlugin,
			RemoteCommandsPlugin,
		))
		.add_systems(Startup, export_remote_loader)
		.run()
}

/// Spawn the remote-control CLI bundle (a [`CliServer`] over [`remote_scene`]),
/// serialize it to the [`scene_output_path`] and exit.
fn export_remote_loader(world: &mut World) -> Result {
	let entity = world.spawn((CliServer::default(), remote_scene())).id();
	export_scene(world, entity)
}
