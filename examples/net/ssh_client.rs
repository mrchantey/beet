//! SSH client example
//!
//! Connects to the ssh_server example, sends a shell command, and prints the response.
//!
//! Make sure the server is running first:
//! ```sh
//! cargo run --example ssh_server --features ssh_server
//! ```
//! Then run:
//! ```sh
//! cargo run --example ssh_client --features ssh_client
//! ```

use beet::net::prelude::*;
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AsyncPlugin::default(),
		))
		.spawn_then((
			SshSession::insert_on_connect("127.0.0.1:8322", "guest", "beet"),
			OnSpawn::observe(on_recv),
		))
		.run();
}

/// Handles all SSH events: sends a command on connect, logs data, and exits on close.
fn on_recv(ev: On<SshRecv>, mut commands: Commands) {
	match ev.event().inner() {
		SshEvent::Connect => {
			info!("SSH session ready, sending command…");
			commands.entity(ev.target()).trigger_target(SshSend(
				SshEvent::text("echo hello from beet\n"),
			));
		}
		SshEvent::Data(_) => {
			if let Some(text) = ev.event().as_str() {
				let trimmed = text.trim();
				info!("Received: {:?}", trimmed);
				if trimmed.contains("hello from beet") {
					commands.write_message(AppExit::Success);
				}
			}
		}
		SshEvent::Close(frame) => {
			info!("Session closed: {:?}", frame.as_ref().map(|f| f.code));
			commands.write_message(AppExit::Success);
		}
		_ => {}
	}
}
