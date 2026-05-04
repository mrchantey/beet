//! SSH shell server example
//!
//! Demonstrates an SSH server that executes shell commands received from clients.
//!
//! Run with:
//! ```sh
//! cargo run --example ssh_server --features ssh_server
//! ```
//! Connect with any SSH client:
//! ```sh
//! ssh -p 2222 guest@127.0.0.1 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
//! ```
//! Or run the companion example:
//! ```sh
//! cargo run --example ssh_client --features ssh_client
//! ```

use beet::net::prelude::*;
use beet::prelude::*;

fn main() -> Result {
	App::new()
		.add_plugins((MinimalPlugins, LogPlugin::default(), SshServerPlugin))
		.spawn_then((
			SshServer::default(),
			OnSpawn::observe(on_client_connected),
			OnSpawn::observe(on_data_recv),
		))
		.run();
	Ok(())
}

/// Per-connection line input buffer.
#[derive(Component, Default)]
struct InputBuffer(Vec<u8>);

/// Sends a welcome banner and inserts the input buffer when a client connects.
fn on_client_connected(
	ev: On<SshClientConnected>,
	mut commands: Commands,
	peers: Query<&SshPeerInfo>,
) {
	// original_target() is the connection entity (event propagated up from child)
	let conn = ev.original_target();
	let greeting = if let Ok(info) = peers.get(conn) {
		if let Some(user) = &info.username {
			format!(
				"Welcome, {}! Type a command and press Enter.\r\n\
				Type 'exit' or Ctrl+C to disconnect.\r\n$ ",
				user
			)
		} else {
			"Welcome! Type a command and press Enter.\r\n\
			Type 'exit' or Ctrl+C to disconnect.\r\n$ "
				.to_owned()
		}
	} else {
		"Welcome! Type a command and press Enter.\r\n$ ".to_owned()
	};
	commands
		.entity(conn)
		.insert(InputBuffer::default())
		.trigger_target(SshDataSend(SshData::text(greeting)));
}

/// Buffers input, echoes typed characters, and executes commands on Enter.
fn on_data_recv(
	ev: On<SshDataRecv>,
	mut commands: Commands,
	mut buffers: Query<&mut InputBuffer>,
) {
	let conn = ev.original_target();
	let Ok(mut buf) = buffers.get_mut(conn) else {
		return;
	};
	let Some(bytes) = ev.event().inner().as_bytes() else {
		return;
	};

	// Ctrl+C — send bye and return (connection will close naturally)
	if bytes.contains(&3u8) {
		commands
			.entity(conn)
			.trigger_target(SshDataSend(SshData::text("\r\nBye!\r\n")));
		return;
	}

	for &byte in bytes {
		match byte {
			// Backspace / DEL — erase last buffered character
			0x7f | 0x08 => {
				if buf.0.pop().is_some() {
					commands.entity(conn).trigger_target(SshDataSend(
						SshData::text("\x08 \x08"),
					));
				}
			}
			// Enter (CR or LF) — execute buffered command
			b'\r' | b'\n' => {
				commands
					.entity(conn)
					.trigger_target(SshDataSend(SshData::text("\r\n")));
				let cmd_str =
					String::from_utf8_lossy(&buf.0).trim().to_string();
				buf.0.clear();

				if cmd_str.is_empty() {
					commands
						.entity(conn)
						.trigger_target(SshDataSend(SshData::text("$ ")));
					continue;
				}

				if cmd_str == "exit" {
					commands.entity(conn).trigger_target(SshDataSend(
						SshData::text("Goodbye!\r\n"),
					));
					return;
				}

				// Execute the command asynchronously and send output back.
				commands.entity(conn).queue_async_local(
					move |entity: AsyncEntity| async move {
						let output = ChildProcess::new("sh")
							.with_args(["-c", &cmd_str])
							.run_async()
							.await;
						let response = match output {
							Ok(out) => {
								// Convert LF to CRLF for SSH raw mode.
								let stdout =
									String::from_utf8_lossy(&out.stdout)
										.replace('\n', "\r\n");
								let stderr =
									String::from_utf8_lossy(&out.stderr)
										.replace('\n', "\r\n");
								format!("{}{}$ ", stdout, stderr)
							}
							Err(err) => format!("error: {}\r\n$ ", err),
						};
						entity
							.trigger_target_then(SshDataSend(SshData::text(
								response,
							)))
							.await;
						Ok(())
					},
				);
			}
			// Printable ASCII — echo and buffer
			c if c >= 0x20 => {
				buf.0.push(c);
				if let Ok(ch) = std::str::from_utf8(&[c]) {
					commands.entity(conn).trigger_target(SshDataSend(
						SshData::text(ch.to_owned()),
					));
				}
			}
			_ => {}
		}
	}
}
