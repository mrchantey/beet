//! SSH TUI demo — an ANSI interactive counter over SSH.
//!
//! Demonstrates an interactive terminal UI served to SSH clients: a
//! [`ChannelTerminal`] per connection buffers its output, and
//! [`terminal_input_bridge`] parses its input bytes into bevy [`KeyboardInput`],
//! which the app reads like any other input source.
//!
//! - Press `+` or `=` to increment the counter
//! - Press `-` to decrement
//! - Press `r` to reset
//! - Press `q` or Ctrl+C to disconnect
//!
//! Run with:
//! ```sh
//! cargo run --example ssh_tui --features ssh_server,tui
//! ```
//! Connect (the default port is [`DEFAULT_SSH_PORT`]):
//! ```sh
//! ssh -p 8339 guest@127.0.0.1 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null
//! ```

use beet::net::prelude::*;
use beet::prelude::*;
use bevy::color::Color;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyCode;
use bevy::input::keyboard::KeyboardInput;

fn main() -> Result {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			// registers the input message types `terminal_input_bridge` writes;
			// headless, which is exactly what a terminal needs.
			bevy::input::InputPlugin,
			SshServerPlugin::default(),
			CharcellPlugin,
		))
		.spawn(SshServer::default())
		// each connection's terminal bytes become bevy input, before InputPlugin
		// folds this frame's messages into `ButtonInput`.
		.add_systems(
			PreUpdate,
			terminal_input_bridge.before(bevy::input::InputSystems),
		)
		// a keypress lands before the frame it paints, so input is never a frame late
		.add_systems(Update, (on_input, render_frame).chain())
		.add_systems(PostUpdate, ssh_write.after(CharcellRenderSet))
		.add_observer(ssh_read)
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

/// One connection's key presses for this frame, folded before they are applied
/// so a single pass over the message stream serves every surface.
///
/// `ctrl`/`key_c` are tracked apart from the typed characters because a modifier
/// arrives as its own press/release around the key it modifies: ctrl+c is
/// "both seen this frame, on the same window", and carries no typed text.
#[derive(Default)]
struct FrameKeys {
	delta: i32,
	reset: bool,
	ctrl: bool,
	key_c: bool,
	quit: bool,
}

/// Applies each connection's key presses to its own counter, closing the session
/// on `q` or ctrl+c.
///
/// [`terminal_input_bridge`] tags every [`KeyboardInput`] with the surface entity
/// that produced it as its `window`, so grouping by window keeps one client's
/// keystrokes off another's counter.
fn on_input(
	mut keys: MessageReader<KeyboardInput>,
	mut commands: Commands,
	mut query: Query<(&mut Counter, &mut Terminal, &mut ChannelTerminal)>,
) -> Result {
	let mut per_window = HashMap::<Entity, FrameKeys>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		let frame = per_window.entry(key.window).or_default();
		match key.key_code {
			KeyCode::ControlLeft | KeyCode::ControlRight => frame.ctrl = true,
			KeyCode::KeyC => frame.key_c = true,
			_ => {}
		}
		match key.text.as_deref() {
			Some("+" | "=") => frame.delta += 1,
			Some("-") => frame.delta -= 1,
			Some("r") => frame.reset = true,
			Some("q") => frame.quit = true,
			_ => {}
		}
	}

	for (window, frame) in per_window {
		let Ok((mut counter, mut terminal, mut channel_terminal)) =
			query.get_mut(window)
		else {
			continue;
		};
		if frame.reset {
			counter.0 = 0;
		}
		counter.0 += frame.delta;
		if !(frame.quit || (frame.ctrl && frame.key_c)) {
			continue;
		}
		// Flush terminal state before closing.
		terminal.restore_config()?;
		terminal.flush()?;
		let output = channel_terminal.drain_write();
		if !output.is_empty() {
			commands
				.entity(window)
				.trigger_target(SshSend(SshEvent::bytes(output)));
		}
		// Send Close to initiate graceful shutdown; despawn happens
		// in ssh_read when the resulting SshRecv(Close) arrives.
		commands
			.entity(window)
			.trigger_target(SshSend(SshEvent::Close(None)));
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
	let entity = ev.target();
	match ev.event().inner() {
		SshEvent::Connect => {}
		SshEvent::RequestPty(pty) => {
			// Insert the terminal now that we know the PTY size.
			commands.entity(entity).insert((
				ChannelTerminal::new(default()),
				DoubleBuffer::new(pty.window.cells),
				Counter::default(),
			));
		}
		SshEvent::Data(bytes) => {
			if let Ok(mut term) = query.get_mut(entity) {
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
