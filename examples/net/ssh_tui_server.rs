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
		.spawn_then((
			SshServer::default(),
			OnSpawn::observe(on_connected),
			OnSpawn::observe(on_data),
			OnSpawn::observe(on_disconnected),
		))
		.run();
	Ok(())
}

/// Per-connection counter state.
#[derive(Component, Default)]
struct Counter(i32);

/// Sends the initial TUI frame and sets up connection state.
fn on_connected(ev: On<SshClientConnected>, mut commands: Commands) {
	let conn = ev.original_target();
	// enter alternate screen and render initial frame
	commands
		.entity(conn)
		.insert(Counter::default())
		.trigger_target(SshDataSend(SshData::text(
			"\x1b[?1049h\x1b[2J".to_owned() + &render_frame(0),
		)));
}

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
		║   Counter: {count:>6}          ║\r\n\
		║  [+/=] inc  [-] dec  [r] reset  [q] quit  ║\r\n\
		╚═══════════════════════════╝\x1b[0m\r\n"
	)
}

/// Handles keyboard input and updates the counter.
fn on_data(
	ev: On<SshDataRecv>,
	mut commands: Commands,
	mut counters: Query<&mut Counter>,
) {
	let conn = ev.original_target();
	let Ok(mut counter) = counters.get_mut(conn) else {
		return;
	};

	match ev.event().inner().as_bytes() {
		Some(b"+" | b"=") => counter.0 += 1,
		Some(b"-") => counter.0 -= 1,
		Some(b"r") => counter.0 = 0,
		// q or Ctrl+C: exit alternate screen and close
		Some(b"q") | Some([3]) => {
			commands
				.entity(conn)
				.trigger_target(SshDataSend(SshData::text("\x1b[?1049l")));
			return;
		}
		_ => return,
	}

	let frame = render_frame(counter.0);
	commands
		.entity(conn)
		.trigger_target(SshDataSend(SshData::text(frame)));
}

/// Cleans up the connection entity on disconnect.
fn on_disconnected(ev: On<SshClientDisconnected>, mut commands: Commands) {
	commands.entity(ev.target()).despawn();
}
