//! SSH TUI demo — an ANSI interactive counter over SSH.
//!
//! Demonstrates an interactive terminal UI served to SSH clients using the
//! beet [`BufferedTerminal`] component for input parsing and output buffering.
//!
//! - Press `+` or `=` to increment the counter
//! - Press `-` to decrement
//! - Press `r` to reset
//! - Press `q` or Ctrl+C to disconnect
//!
//! Run with:
//! ```sh
//! cargo run --example ssh_tui --features ssh_server,node,termion
//! ```
//! Connect:
//! ```sh
//! ssh -p 2222 guest@127.0.0.1 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
//! ```

use beet::net::prelude::*;
use beet::prelude::*;
use std::io::Write;
use termion::event::Event;
use termion::event::Key;

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

/// Renders a colour-cycling counter panel.
fn render_frame(count: i32, terminal: &mut BufferedTerminal) -> Result {
	use termion::clear;
	use termion::color;
	use termion::cursor;
	let (r, g, b): (u8, u8, u8) = match count.rem_euclid(3) {
		0 => (220, 50, 50),
		1 => (50, 200, 50),
		_ => (50, 100, 220),
	};
	write!(
		terminal.writer,
		"{}{}{}{}╔═══════════════════════════╗\r\n\
		║   beet SSH TUI demo       ║\r\n\
		║   Counter: {:<11}    ║\r\n\
		║  [+/=] inc  [-] dec       ║\r\n\
		║  [r] reset  [q] quit      ║\r\n\
		╚═══════════════════════════╝{}",
		cursor::Goto(1, 1),
		clear::All,
		color::Fg(color::Rgb(r, g, b)),
		color::Bg(color::Reset),
		count,
		color::Fg(color::Reset),
	)
	.map_err(|e| bevyhow!("{e}"))
}

/// Handles all SSH events for a connection.
fn on_recv(
	ev: On<SshRecv>,
	mut commands: Commands,
	mut counters: Query<&mut Counter>,
	mut terminals: Query<&mut BufferedTerminal>,
) {
	let conn = ev.original_target();

	match ev.event().inner() {
		SshEvent::Connect => {}
		SshEvent::RequestPty(pty) => {
			// Insert the terminal now that we know the PTY size.
			let size = pty.window.cells;
			let mut terminal = BufferedTerminal::new_buffered(size);
			// Send initial frame.
			let _ = render_frame(0, &mut terminal);
			let output = terminal.take_output();
			commands
				.entity(conn)
				.insert((terminal, Counter::default()))
				.trigger_target(SshSend(SshEvent::bytes(output)));
		}
		SshEvent::Data(bytes) => {
			// Parse incoming bytes into termion events.
			let events = BufferedTerminal::parse_bytes(bytes);

			let mut quit = false;
			if let Ok(mut counter) = counters.get_mut(conn) {
				for ev in &events {
					match ev {
						Event::Key(Key::Char('+') | Key::Char('=')) => {
							counter.0 += 1
						}
						Event::Key(Key::Char('-')) => counter.0 -= 1,
						Event::Key(Key::Char('r')) => counter.0 = 0,
						Event::Key(Key::Char('q')) => quit = true,
						Event::Key(Key::Ctrl('c')) => quit = true,
						_ => {}
					}
				}
			}

			if quit {
				commands
					.entity(conn)
					.trigger_target(SshSend(SshEvent::text("\x1b[?1049l")))
					.trigger_target(SshSend(SshEvent::Close(None)));
				return;
			}

			// Re-render and send updated frame.
			let count = counters.get(conn).map(|c| c.0).unwrap_or(0);
			if let Ok(mut terminal) = terminals.get_mut(conn) {
				let _ = render_frame(count, &mut terminal);
				let output = terminal.take_output();
				commands
					.entity(conn)
					.trigger_target(SshSend(SshEvent::bytes(output)));
			}
		}
		SshEvent::Close(_) => {
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

	/// Pressing '+' increments the TUI counter from 0 to 1.
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
