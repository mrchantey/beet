//! SSH client example
//!
//! Demonstrates connecting to an SSH server and sending/receiving data.
//!
//! Make sure the server is running first:
//! ```sh
//! cargo run --example ssh_server --features ssh_server
//! ```
//!
//! Run with:
//! ```sh
//! cargo run --example ssh_client --features ssh_client
//! ```

use beet::net::prelude::*;
use beet::prelude::*;

const MESSAGES: &[&str] = &["hello from beet", "second message", "goodbye"];

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AsyncPlugin::default(),
		))
		.insert_resource(MessageIndex(0))
		.spawn_then((
			SshSession::insert_on_connect("127.0.0.1:2222", "guest", "beet"),
			OnSpawn::observe(on_ready),
			OnSpawn::observe(on_recv),
		))
		.run();
	info!("Done");
}

/// Tracks which message we're sending next.
#[derive(Resource)]
struct MessageIndex(usize);

/// Sends the first message once the session is connected.
fn on_ready(
	ev: On<SshSessionReady>,
	mut commands: Commands,
	mut idx: ResMut<MessageIndex>,
) {
	info!("SSH session ready, sending messages…");
	if let Some(msg) = MESSAGES.get(idx.0) {
		commands
			.entity(ev.target())
			.trigger_target(SshDataSend(SshData::text(*msg)));
		idx.0 += 1;
	}
}

/// Logs responses and sends the next message in sequence, then exits.
fn on_recv(
	ev: On<SshDataRecv>,
	mut commands: Commands,
	mut idx: ResMut<MessageIndex>,
) {
	match ev.event().inner() {
		SshData::Bytes(_) => {
			if let Some(text) = ev.event().inner().as_str() {
				info!("Received: {}", text.trim());
				// send the next queued message
				if let Some(msg) = MESSAGES.get(idx.0) {
					commands
						.entity(ev.target())
						.trigger_target(SshDataSend(SshData::text(*msg)));
					idx.0 += 1;
				} else {
					// all messages sent and at least one echoed, we're done
					info!("All messages exchanged, disconnecting.");
					commands.write_message(AppExit::Success);
				}
			}
		}
		SshData::Exit(code) => {
			info!("Server exit code: {}", code);
			commands.write_message(AppExit::Success);
		}
	}
}
