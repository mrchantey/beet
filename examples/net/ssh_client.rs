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
			SshSession::insert_on_connect("127.0.0.1:2222", "guest", "beet"),
			OnSpawn::observe(on_ready),
			OnSpawn::observe(on_recv),
		))
		.run();
}

/// Sends a shell command once the session is connected.
fn on_ready(ev: On<SshSessionReady>, mut commands: Commands) {
	info!("SSH session ready, sending command…");
	commands
		.entity(ev.target())
		.trigger_target(SshDataSend(SshData::text("echo hello from beet\n")));
}

/// Logs responses and exits after seeing the expected command output.
fn on_recv(ev: On<SshDataRecv>, mut commands: Commands) {
	match ev.event().inner() {
		SshData::Bytes(_) => {
			if let Some(text) = ev.event().inner().as_str() {
				let trimmed = text.trim();
				info!("Received: {:?}", trimmed);
				// Exit once the echo output arrives
				if trimmed.contains("hello from beet") {
					commands.write_message(AppExit::Success);
				}
			}
		}
		SshData::Exit(code) => {
			info!("Session exit code: {}", code);
			commands.write_message(AppExit::Success);
		}
	}
}
