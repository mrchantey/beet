//! SSH TUI demo — an ANSI interactive counter over SSH.
//!
//! Demonstrates serving a toy interactive UI to any SSH client.
//!
//! - Press `+` or `=` to increment the counter
//! - Press `-` to decrement
//! - Press `r` to reset
//! - Press `q` or Ctrl+C to disconnect
//!
//! Run with:
//! ```sh
//! cargo run --example ssh_tui_server --features ssh_server
//! ```
//! Connect:
//! ```sh
//! ssh -p 2222 guest@127.0.0.1 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
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
		.spawn_then((SshServer::default(), OnSpawn::observe(on_recv)))
		.run();
	Ok(())
}

/// Per-connection counter state.
#[derive(Component, Default)]
struct Counter(i32);

/// Renders a colour-cycling counter panel using ANSI escape codes.
fn render_frame(count: i32) -> String {
	let color = match count.rem_euclid(3) {
		0 => "\x1b[31m", // red
		1 => "\x1b[32m", // green
		_ => "\x1b[34m", // blue
	};
	format!(
		"\x1b[H\x1b[2J{color}\
		╔═══════════════════════════╗\r\n\
		║   beet SSH TUI demo       ║\r\n\
		║   Counter: {count:<11}    ║\r\n\
		║  [+/=] inc  [-] dec       ║\r\n\
		║  [r] reset  [q] quit      ║\r\n\
		╚═══════════════════════════╝\x1b[0m\r\n"
	)
}

/// Handles all SSH events: connect, keyboard input, and disconnect.
fn on_recv(
	ev: On<SshRecv>,
	mut commands: Commands,
	mut counters: Query<&mut Counter>,
) {
	let conn = ev.original_target();

	match ev.event().inner() {
		SshEvent::Connect => {
			// Enter alternate screen and render initial frame.
			commands
				.entity(conn)
				.insert(Counter::default())
				.trigger_target(SshSend(SshEvent::text(
					"\x1b[?1049h\x1b[2J".to_owned() + &render_frame(0),
				)));
		}
		SshEvent::Data(bytes) => {
			let Ok(mut counter) = counters.get_mut(conn) else {
				return;
			};
			match bytes.as_ref() {
				b"+" | b"=" => counter.0 += 1,
				b"-" => counter.0 -= 1,
				b"r" => counter.0 = 0,
				// q or Ctrl+C: exit alternate screen and disconnect
				b"q" | [3] => {
					commands
						.entity(conn)
						.trigger_target(SshSend(SshEvent::text("\x1b[?1049l")))
						.trigger_target(SshSend(SshEvent::Close(None)));
					return;
				}
				_ => return,
			}
			let frame = render_frame(counter.0);
			commands
				.entity(conn)
				.trigger_target(SshSend(SshEvent::text(frame)));
		}
		SshEvent::Close(_) => {
			// Clean up the connection entity on disconnect.
			commands.entity(conn).despawn();
		}
		_ => {}
	}
}

#[cfg(test)]
#[cfg(all(
	feature = "ssh_server",
	feature = "ssh_client",
	not(target_arch = "wasm32")
))]
mod tests {
	use super::*;
	use beet::net::prelude::*;
	use beet::prelude::*;
	use std::time::Duration;

	/// Verifies that pressing '+' increments the TUI counter from 0 to 1.
	#[test]
	fn counter_increments() {
		let (server, on_spawn) = SshServer::new_test();
		let addr = server.local_address();
		let store = Store::<Option<String>>::default();
		let store_clone = store.clone();
		let initial_received = Store::<bool>::default();
		let initial_clone = initial_received.clone();

		// Start the TUI server in a background thread.
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SshServerPlugin::default()));
			app.world_mut().spawn((
				server,
				on_spawn,
				OnSpawn::observe(on_recv),
			));
			app.run();
		});

		// Give the server time to start.
		std::thread::sleep(Duration::from_millis(300));

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin::default()));
		app.world_mut()
			.spawn(SshSession::insert_on_connect(&addr, "guest", "beet"))
			.observe_any(move |ev: On<SshRecv>, mut commands: Commands| {
				match ev.event().inner() {
					SshEvent::Data(_) => {
						if let Some(text) = ev.event().as_str() {
							if !initial_clone.get() && text.contains("Counter:")
							{
								// Initial frame received — send '+'.
								initial_clone.set(true);
								commands.entity(ev.target()).trigger_target(
									SshSend(SshEvent::text("+")),
								);
							} else if initial_clone.get()
								&& text.contains("Counter:")
								&& text.contains("     1")
							{
								// Updated frame shows count = 1.
								store_clone.set(Some(text.to_owned()));
								commands.write_message(AppExit::Success);
							}
						}
					}
					SshEvent::Close(_) => {
						commands.write_message(AppExit::Success);
					}
					_ => {}
				}
			});
		app.run();

		store.get().is_some().xpect_true();
	}
}
