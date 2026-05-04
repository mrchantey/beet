//! SSH echo server example
//!
//! Demonstrates an SSH server that accepts connections, sends a welcome
//! banner, and echoes all received data back with a `> ` prefix.
//!
//! Run with:
//! ```sh
//! cargo run --example ssh_server --features ssh_server
//! ```
//! Connect with any SSH client:
//! ```sh
//! ssh -p 2222 guest@127.0.0.1 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
//! password: beet
//! # or run the companion example:
//! cargo run --example ssh_client --features ssh_client
//! ```

use beet::net::prelude::*;
use beet::prelude::*;

fn main() -> Result {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			SshServerPlugin::default(),
		))
		.spawn_then((
			SshServer::default(),
			// send welcome banner when a client opens a session
			OnSpawn::observe(on_client_connected),
			// echo received data back with a prefix
			OnSpawn::observe(on_data_recv),
		))
		.run();
	Ok(())
}

/// Sends a welcome banner when a new SSH client connects.
fn on_client_connected(ev: On<SshClientConnected>, mut commands: Commands) {
	let conn = ev.original_target();
	commands.entity(conn).trigger_target(SshDataSend(
		SshData::text("Welcome to beet SSH!\r\nType anything and it will be echoed back. Press Ctrl+C to disconnect.\r\n"),
	));
}

/// Echoes received data back prefixed with `> `.
fn on_data_recv(ev: On<SshDataRecv>, mut commands: Commands) {
	if let Some(text) = ev.event().inner().as_str() {
		// Ctrl+C disconnects (byte 0x03)
		if text.as_bytes() == [3] {
			return;
		}
		let echo = format!("> {}\r\n", text.trim_end_matches(['\r', '\n']));
		commands
			.entity(ev.original_target())
			.trigger_target(SshDataSend(SshData::text(echo)));
	}
}
