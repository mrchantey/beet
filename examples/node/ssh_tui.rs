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
//! cargo run --example ssh_tui --features ssh_server,terminal
//! ```
//! Connect:
//! ```sh
//! ssh -p 2222 guest@127.0.0.1 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
//! ```

use beet::net::prelude::*;
use beet::prelude::*;
use bevy::color::Color;

fn main() -> Result {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			SshServerPlugin::default(),
			CharcellPlugin,
		))
		.spawn_then(SshServer::default())
		.add_systems(Update, render_frame)
		.add_systems(PostUpdate, ssh_write.after(CharcellRenderStep))
		.add_observer(ssh_read)
		.add_observer(on_input)
		.run();
	Ok(())
}

/// Per-connection counter state.
#[derive(Component, Default)]
struct Counter(i32);


/// Renders a colour-cycling counter panel.
fn render_frame(mut query: Populated<(&mut Terminal, &Counter)>) -> Result {
	for (mut terminal, counter) in query.iter_mut() {
		render(counter.0, terminal.writer_mut())?;
	}
	Ok(())
}

fn render(count: i32, mut writer: impl std::io::Write) -> Result {
	let color = match count.rem_euclid(3) {
		0 => Color::srgb_u8(220, 50, 50),
		1 => Color::srgb_u8(50, 200, 50),
		_ => Color::srgb_u8(50, 100, 220),
	};
	// Move to origin and clear screen.
	writer.write_all(escape::CURSOR_HOME.as_bytes())?;
	writer.write_all(escape::ERASE_ALL.as_bytes())?;
	// Write foreground colour and reset background.
	escape::foreground(&mut writer, color)?;
	writer.write_all(escape::RESET_BG.as_bytes())?;
	// Write the frame content.
	write!(
		writer,
		"╔═══════════════════════════╗\r\n\
		 ║   beet SSH TUI demo       ║\r\n\
		 ║   Counter: {:<11}    ║\r\n\
		 ║  [+/=] inc  [-] dec       ║\r\n\
		 ║  [r] reset  [q] quit      ║\r\n\
		 ╚═══════════════════════════╝",
		count,
	)?;
	writer.write_all(escape::RESET_FG.as_bytes())?;
	Ok(())
}


fn on_input(
	ev: On<TerminalEvent>,
	mut commands: Commands,
	mut query: Query<(&mut Counter, &mut Terminal, &mut ChannelTerminal)>,
) -> Result {
	use TerminalEvent::*;
	let Ok((mut counter, mut terminal, mut channel_terminal)) =
		query.get_mut(ev.target())
	else {
		return Ok(());
	};
	match ev.event() {
		Key(key) if matches!(key.char, Some('+' | '=')) => counter.0 += 1,
		Key(key) if key.char == Some('-') => counter.0 -= 1,
		Key(key) if key.char == Some('r') => counter.0 = 0,
		Key(key) if key.char == Some('q') || key == &KeyPress::CTRL_C => {
			// perform a restore flush and despawn
			terminal.restore_config()?;
			terminal.flush()?;
			let output = channel_terminal.drain_write();
			if !output.is_empty() {
				commands
					.entity(ev.target())
					.trigger_target(SshSend(SshEvent::bytes(output)));
			}

			// TODO this currently causes server to crash, despawned entity..
			// possibly due to propagating events?
			commands.entity(ev.target()).despawn();
		}

		_ => {}
	}
	Ok(())
}

fn ssh_write(
	mut commands: Commands,
	mut query: Query<(Entity, &mut ChannelTerminal)>,
) -> Result {
	for (entity, mut terminal) in query.iter_mut() {
		let output = terminal.drain_write();
		if !output.is_empty() {
			commands
				.entity(entity)
				.trigger_target(SshSend(SshEvent::bytes(output)));
		}
	}

	Ok(())
}

/// Handles all SSH events for a connection.
fn ssh_read(
	ev: On<SshRecv>,
	mut commands: Commands,
	mut query: Query<&mut ChannelTerminal>,
) -> Result {
	let entity = ev.original_target();
	match ev.event().inner() {
		SshEvent::Connect => {}
		SshEvent::RequestPty(pty) => {
			// Insert the terminal now that we know the PTY size.
			commands.entity(entity).insert((
				ChannelTerminal::new(pty.window.cells, default()),
				Counter::default(),
			));
		}
		SshEvent::Data(bytes) => {
			if let Ok(mut term) = query.get_mut(ev.target()) {
				term.send_input(bytes)?;
			}
		}
		SshEvent::Close(_) => {
			commands.entity(entity).despawn();
		}
		_ => {}
	}
	Ok(())
}
