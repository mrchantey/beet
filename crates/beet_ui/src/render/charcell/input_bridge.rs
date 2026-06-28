//! Terminal input to bevy input bridge.
//!
//! Translates the [`InputParser`]'s [`TerminalEvent`]s into bevy's own input
//! messages ([`KeyboardInput`], [`MouseButtonInput`], [`CursorMoved`],
//! [`MouseWheel`]) so the whole app and the future native renderer consume one
//! unified input path with zero terminal awareness. The terminal host entity
//! doubles as the `window` surface on every emitted event.
//!
//! Terminal input is a stream of discrete keystrokes/clicks with no separate
//! key-up, so each key emits a paired Pressed+Released within the frame: the
//! Pressed message feeds the [`FocusPlugin`](crate::prelude::FocusPlugin) stream
//! and `just_pressed`, the Released keeps `ButtonInput` from latching keys down.

use super::*;
use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyCode;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input::mouse::MouseWheel;
use bevy::input::touch::TouchPhase;
use bevy::math::Vec2;
use bevy::window::CursorMoved;

/// How many lines one mouse-wheel notch scrolls (mirrors the old TUI constant).
const WHEEL_LINES: f32 = 3.;

/// ECS system: read each terminal's parsed input and emit bevy input messages.
///
/// Replaces the old `TerminalEvent`-trigger path: keys become [`KeyboardInput`],
/// mouse buttons [`MouseButtonInput`], motion [`CursorMoved`], wheel
/// [`MouseWheel`], and a resize reallocates the host [`DoubleBuffer`]. The host
/// entity is the `window` surface for every event.
pub fn terminal_input_bridge(
	mut keyboard: MessageWriter<KeyboardInput>,
	mut mouse_button: MessageWriter<MouseButtonInput>,
	mut cursor: MessageWriter<CursorMoved>,
	mut wheel: MessageWriter<MouseWheel>,
	mut query: Populated<(Entity, &mut Terminal, Option<&mut DoubleBuffer>)>,
) -> Result {
	for (surface, mut terminal, mut buffer) in query.iter_mut() {
		for event in terminal.read_events()? {
			match event {
				TerminalEvent::Key(key) => {
					for input in keyboard_inputs(key, surface) {
						keyboard.write(input);
					}
				}
				TerminalEvent::Mouse(mouse) => {
					emit_mouse(
						mouse,
						surface,
						&mut mouse_button,
						&mut cursor,
						&mut wheel,
					);
				}
				TerminalEvent::Paste(text) => {
					// paste has no first-class bevy event; replay it as a character
					// stream so a focused input receives it (documented choice).
					for ch in text.chars() {
						for input in keyboard_inputs(
							KeyPress::with_char(
								char_to_keycode(ch),
								KeyModifier::empty(),
								Some(ch),
							),
							surface,
						) {
							keyboard.write(input);
						}
					}
				}
				TerminalEvent::Resize(size) => {
					if let Some(buffer) = buffer.as_mut() {
						buffer.resize(size);
					}
				}
				TerminalEvent::Unsupported(_) => {}
			}
		}
	}
	Ok(())
}

/// The bevy [`KeyboardInput`] messages for one terminal [`KeyPress`]: any active
/// modifier keys pressed, the main key pressed then released, then the modifiers
/// released, all within the frame (terminal keystrokes are discrete).
fn keyboard_inputs(key: KeyPress, window: Entity) -> Vec<KeyboardInput> {
	let mut inputs = Vec::new();
	let modifiers = modifier_keycodes(*key.modifier());
	for &code in &modifiers {
		inputs.push(named_input(
			code,
			modifier_key(code),
			ButtonState::Pressed,
			window,
		));
	}
	let logical = logical_key(&key);
	let text = key.char.map(|c| c.to_string().into());
	inputs.push(KeyboardInput {
		key_code: *key.key(),
		logical_key: logical.clone(),
		state: ButtonState::Pressed,
		text: text.clone(),
		repeat: false,
		window,
	});
	inputs.push(KeyboardInput {
		key_code: *key.key(),
		logical_key: logical,
		state: ButtonState::Released,
		text,
		repeat: false,
		window,
	});
	for &code in modifiers.iter().rev() {
		inputs.push(named_input(
			code,
			modifier_key(code),
			ButtonState::Released,
			window,
		));
	}
	inputs
}

/// A [`KeyboardInput`] for a named (non-text) key.
fn named_input(
	key_code: KeyCode,
	logical_key: Key,
	state: ButtonState,
	window: Entity,
) -> KeyboardInput {
	KeyboardInput {
		key_code,
		logical_key,
		state,
		text: None,
		repeat: false,
		window,
	}
}

/// The logical [`Key`] for a key press: a named key (Enter/Tab/arrows/...) when
/// the physical code is one, else the typed character, else unidentified.
///
/// A named key takes precedence even though the parser gives Enter/Tab a `\n`/`\t`
/// char, because their logical key is `Key::Enter`/`Key::Tab`, not a character.
fn logical_key(key: &KeyPress) -> Key {
	let named = named_logical_key(*key.key());
	if !matches!(named, Key::Unidentified(_)) {
		return named;
	}
	if let Some(c) = key.char {
		// a control combo (eg ctrl+c) carries a char but should not type it; only
		// plain/shifted printable chars become text.
		if !key
			.modifier()
			.intersects(KeyModifier::CTRL | KeyModifier::ALT)
		{
			return Key::Character(c.to_string().into());
		}
	}
	named
}

/// Map a physical [`KeyCode`] to its named logical [`Key`], for keys that carry
/// no typed text (navigation, editing, function keys). Falls back to
/// [`Key::Unidentified`] for an unmapped code.
fn named_logical_key(code: KeyCode) -> Key {
	match code {
		KeyCode::Enter => Key::Enter,
		KeyCode::Backspace => Key::Backspace,
		KeyCode::Tab => Key::Tab,
		KeyCode::Space => Key::Space,
		KeyCode::Delete => Key::Delete,
		KeyCode::Escape => Key::Escape,
		KeyCode::Home => Key::Home,
		KeyCode::End => Key::End,
		KeyCode::PageUp => Key::PageUp,
		KeyCode::PageDown => Key::PageDown,
		KeyCode::ArrowUp => Key::ArrowUp,
		KeyCode::ArrowDown => Key::ArrowDown,
		KeyCode::ArrowLeft => Key::ArrowLeft,
		KeyCode::ArrowRight => Key::ArrowRight,
		_ => Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
	}
}

/// The physical modifier [`KeyCode`]s active in a [`KeyModifier`] set.
fn modifier_keycodes(modifier: KeyModifier) -> Vec<KeyCode> {
	let mut codes = Vec::new();
	if modifier.contains(KeyModifier::CTRL) {
		codes.push(KeyCode::ControlLeft);
	}
	if modifier.contains(KeyModifier::ALT) {
		codes.push(KeyCode::AltLeft);
	}
	if modifier.contains(KeyModifier::SHIFT) {
		codes.push(KeyCode::ShiftLeft);
	}
	codes
}

/// The logical [`Key`] for a modifier [`KeyCode`].
fn modifier_key(code: KeyCode) -> Key {
	match code {
		KeyCode::ControlLeft => Key::Control,
		KeyCode::AltLeft => Key::Alt,
		KeyCode::ShiftLeft => Key::Shift,
		_ => Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
	}
}

/// Emit the bevy messages for one terminal mouse event: a [`CursorMoved`] for the
/// new position plus, per kind, a [`MouseButtonInput`] (press/release) or a
/// [`MouseWheel`] (scroll). Motion-only events emit just the cursor move.
fn emit_mouse(
	mouse: MouseEvent,
	window: Entity,
	button: &mut MessageWriter<MouseButtonInput>,
	cursor: &mut MessageWriter<CursorMoved>,
	wheel: &mut MessageWriter<MouseWheel>,
) {
	// the surface is a 1:1 cell grid, so cell coordinates cast straight to pixels.
	let position = Vec2::new(mouse.position.x as f32, mouse.position.y as f32);
	cursor.write(CursorMoved {
		window,
		position,
		delta: None,
	});
	match mouse.kind {
		MouseEventKind::Press(b) => {
			button.write(MouseButtonInput {
				button: b,
				state: ButtonState::Pressed,
				window,
			});
		}
		MouseEventKind::Release(b) => {
			button.write(MouseButtonInput {
				button: b,
				state: ButtonState::Released,
				window,
			});
		}
		MouseEventKind::Scroll(dir) => {
			let (x, y) = match dir {
				ScrollDirection::Up => (0., WHEEL_LINES),
				ScrollDirection::Down => (0., -WHEEL_LINES),
				ScrollDirection::Left => (-WHEEL_LINES, 0.),
				ScrollDirection::Right => (WHEEL_LINES, 0.),
			};
			wheel.write(MouseWheel {
				unit: MouseScrollUnit::Line,
				x,
				y,
				window,
				phase: TouchPhase::Moved,
			});
		}
		// motion/drag: the cursor move above is the whole signal
		MouseEventKind::Move | MouseEventKind::Drag(_) => {}
	}
}

/// ECS system: resize a [`StdioTerminal`]'s [`DoubleBuffer`] to the real tty size
/// when it changes, the cross-platform stand-in for a `SIGWINCH` handler.
///
/// Polls `terminal_ext::size()` each frame and reallocates on a change, forcing a
/// full repaint. Only [`StdioTerminal`]s are polled; a [`ChannelTerminal`] has a
/// fixed, caller-controlled size (and is resized via `DoubleBuffer::resize`
/// directly in tests), so it is left alone.
pub fn resize_stdio_buffers(
	mut query: Query<&mut DoubleBuffer, With<StdioTerminal>>,
) {
	let size = terminal_ext::size();
	for mut buffer in query.iter_mut() {
		if buffer.current_buffer().size() != size {
			buffer.resize(size);
		}
	}
}

/// ECS system: exit the process on ctrl+c from the local stdio surface only.
///
/// Reads the per-surface [`KeyboardInput`] stream (each event carries its source
/// `window`): a terminal ctrl+c arrives as a Control press bracketing a `C` press
/// on the same window. The raw-mode terminal never receives an OS `SIGINT` (the
/// kernel delivers ctrl+c as a `0x03` byte instead), so this is the only thing
/// that exits a raw-mode app.
///
/// The surface is identified *positively* by [`StdioTerminal`]: only the local
/// terminal exits the process. An SSH session ([`ChannelTerminal`]) handles its
/// ctrl+c as a per-session close (`close_session_on_ctrl_c`), never a global
/// [`AppExit`] that would tear down every other session. A [`StdioTerminal`] that
/// opts out via [`ctrl_c_exit`](StdioTerminal::ctrl_c_exit)`= false` is also left
/// alone, so an app can handle ctrl+c itself.
///
/// The positive match is load-bearing: every SSH client sends a closing ctrl+c,
/// and that keypress and the session despawn race within a frame. A negative
/// "not a [`ChannelTerminal`]" test misfires the instant the despawn (or a not-yet
/// built pre-pty surface, or a reused entity index) leaves the window without a
/// [`ChannelTerminal`], killing the whole multi-tenant server. Keying on
/// [`StdioTerminal`] — which an SSH window never carries — is immune to that race.
pub fn exit_on_ctrl_c(
	mut keys: MessageReader<KeyboardInput>,
	stdio: Query<&StdioTerminal>,
	mut exit: MessageWriter<AppExit>,
) {
	// group this frame's pressed keys by window: (ctrl seen, c seen).
	let mut per_window = HashMap::<Entity, (bool, bool)>::default();
	for key in keys.read().filter(|key| key.state == ButtonState::Pressed) {
		let entry = per_window.entry(key.window).or_default();
		match key.key_code {
			KeyCode::ControlLeft | KeyCode::ControlRight => entry.0 = true,
			KeyCode::KeyC => entry.1 = true,
			_ => {}
		}
	}
	for (window, (ctrl, c)) in per_window {
		// a remote (channel) surface lacks a StdioTerminal and closes its session
		// elsewhere; a local stdio surface exits unless it opts out.
		if ctrl && c && stdio.get(window).is_ok_and(|stdio| stdio.ctrl_c_exit()) {
			exit.write(AppExit::Success);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use crate::render::charcell::test_host::TestHost;
	use bevy::app::AppExit;

	/// The single pressed [`KeyboardInput`] for a logical key (the bridge emits a
	/// matching Released too).
	fn pressed_key(host: &TestHost) -> Option<KeyboardInput> {
		host.messages::<KeyboardInput>()
			.into_iter()
			.find(|input| input.state == ButtonState::Pressed)
	}

	/// A real ctrl+c byte through the full [`CharcellTuiPlugin`] pipeline on a
	/// LOCAL surface (a [`StdioTerminal`]) exits the process: its buffered
	/// headless [`Terminal`] is swapped for a channel-backed one so we can inject
	/// a raw `0x03` byte, which parses to a Control+`C` press pair that
	/// [`exit_on_ctrl_c`] turns into an [`AppExit`].
	#[beet_core::test]
	fn ctrl_c_byte_exits_local_surface() {
		use crate::render::charcell::terminal::ChannelTerminal;
		// safety: single-threaded test; the buffered headless `StdioTerminal` skips
		// the tty/raw-mode setup its `on_add` would otherwise run.
		unsafe { env_ext::set_var("BEET_HEADLESS", "1") };
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, CharcellTuiPlugin));
		// the channel-backed Terminal lets us feed bytes into the real reader.
		let (mut channel, terminal) =
			ChannelTerminal::new(TerminalConfig::default());
		let surface = app
			.world_mut()
			.spawn((StdioTerminal::default(), DoubleBuffer::new(UVec2::new(40, 12))))
			.id();
		// flush the StdioTerminal on_add hook (it inserts a buffered Terminal),
		// then overwrite that Terminal with the channel-backed one.
		app.update();
		app.world_mut().entity_mut(surface).insert(terminal);
		unsafe { env_ext::remove_var("BEET_HEADLESS") };
		app.update();
		channel.send_input(&[0x03]).unwrap();
		app.update();
		app.world()
			.resource::<Messages<AppExit>>()
			.iter_current_update_messages()
			.count()
			.xpect_greater_than(0);
	}

	/// A character keypress maps to a `Key::Character` logical key and the right
	/// physical `KeyCode`, Pressed.
	#[beet_core::test]
	fn char_key_maps_to_keyboard_input() {
		let mut host = TestHost::new();
		host.send_input(b"a");
		host.step();
		let key = pressed_key(&host).expect("a pressed KeyboardInput");
		key.key_code.xpect_eq(KeyCode::KeyA);
		(key.logical_key == Key::Character("a".into())).xpect_true();
		key.text.xpect_eq(Some("a".into()));
	}

	/// Named keys (enter, backspace, arrows) map to their named logical keys.
	#[beet_core::test]
	fn named_keys_map_to_named_logical_keys() {
		let cases: [(&[u8], KeyCode, Key); 4] = [
			(b"\r", KeyCode::Enter, Key::Enter),
			(b"\x7f", KeyCode::Backspace, Key::Backspace),
			(b"\x1b[A", KeyCode::ArrowUp, Key::ArrowUp),
			(b"\x1b[D", KeyCode::ArrowLeft, Key::ArrowLeft),
		];
		for (bytes, code, key) in cases {
			let mut host = TestHost::new();
			host.send_input(bytes);
			host.step();
			let input = pressed_key(&host).expect("a pressed KeyboardInput");
			input.key_code.xpect_eq(code);
			(input.logical_key == key).xpect_true();
		}
	}

	/// An SGR mouse press emits a Left `MouseButtonInput` (Pressed) and a
	/// `CursorMoved` at the 0-indexed cell.
	#[beet_core::test]
	fn sgr_mouse_press_emits_button_and_cursor() {
		let mut host = TestHost::new();
		// SGR press button 0 (left) at 1-indexed (5,3) -> 0-indexed cell (4,2)
		host.send_input(b"\x1b[<0;5;3M");
		host.step();
		let button = host
			.messages::<MouseButtonInput>()
			.into_iter()
			.next()
			.expect("a MouseButtonInput");
		(button.button == MouseButton::Left).xpect_true();
		(button.state == ButtonState::Pressed).xpect_true();
		let cursor = host
			.messages::<CursorMoved>()
			.into_iter()
			.next()
			.expect("a CursorMoved");
		cursor.position.xpect_eq(Vec2::new(4., 2.));
	}

	/// An SGR wheel-up emits a line `MouseWheel` with positive `y`; wheel-down
	/// negative.
	#[beet_core::test]
	fn wheel_emits_mouse_wheel() {
		let mut host = TestHost::new();
		host.send_input(b"\x1b[<64;1;1M"); // scroll up
		host.step();
		let wheel = host
			.messages::<MouseWheel>()
			.into_iter()
			.next()
			.expect("a MouseWheel");
		(wheel.y > 0.).xpect_true();

		let mut host = TestHost::new();
		host.send_input(b"\x1b[<65;1;1M"); // scroll down
		host.step();
		let wheel = host.messages::<MouseWheel>().into_iter().next().unwrap();
		(wheel.y < 0.).xpect_true();
	}

	/// A focused `<input>`-like entity receives typed text through the unified
	/// keyboard path: the bridge feeds `FocusPlugin`, which edits the `Value`.
	#[beet_core::test]
	fn focus_plugin_receives_typed_text() {
		let mut host = TestHost::new();
		// bind the field to the host surface (input_bridge tags events with the host
		// entity as their window), so the per-surface focus path delivers the text.
		let surface = host.host;
		let field = host
			.app
			.world_mut()
			.spawn((Focus, Value::str(""), RenderSurface(surface)))
			.id();
		host.send_input(b"hi");
		host.step();
		host.app
			.world()
			.get::<Value>(field)
			.unwrap()
			.clone()
			.xpect_eq(Value::str("hi"));
	}

	/// ctrl+c on a remote (channel) surface does NOT exit the process: an SSH
	/// session's ctrl+c is a per-session close, never a global `AppExit` that would
	/// tear down every other session.
	#[beet_core::test]
	fn ctrl_c_does_not_exit_remote_surface() {
		let mut host = TestHost::new(); // a ChannelTerminal surface
		host.send_input(&[0x03]); // ctrl+c
		host.step();
		host.messages::<AppExit>().is_empty().xpect_true();
	}

	/// Minimal app running just [`exit_on_ctrl_c`], plus a ctrl+c from `window`.
	fn ctrl_c_from(window: impl FnOnce(&mut World) -> Entity) -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<KeyboardInput>()
			.add_systems(Update, exit_on_ctrl_c);
		let window = window(app.world_mut());
		// ctrl+c arrives as a ControlLeft press bracketing a KeyC press.
		for key_code in [KeyCode::ControlLeft, KeyCode::KeyC] {
			app.world_mut().write_message(KeyboardInput {
				key_code,
				logical_key: Key::Character("c".into()),
				state: ButtonState::Pressed,
				text: None,
				repeat: false,
				window,
			});
		}
		app.update();
		app
	}

	fn exited(app: &App) -> bool {
		!app.world()
			.resource::<Messages<AppExit>>()
			.iter_current_update_messages()
			.count()
			.eq(&0)
	}

	/// ctrl+c on the local stdio surface exits the process.
	#[beet_core::test]
	fn ctrl_c_exits_local_surface() {
		// safety: single-threaded test; the buffered headless `StdioTerminal` skips
		// the tty/raw-mode setup its `on_add` would otherwise run.
		unsafe { env_ext::set_var("BEET_HEADLESS", "1") };
		let app =
			ctrl_c_from(|world| world.spawn(StdioTerminal::default()).id());
		unsafe { env_ext::remove_var("BEET_HEADLESS") };
		exited(&app).xpect_true();
	}

	/// Regression (multi-tenant SSH crash): a ctrl+c from a session window that is
	/// *not* a [`StdioTerminal`] must never exit the process — not even when the
	/// window carries no [`ChannelTerminal`] (a session mid-teardown, before its
	/// pty, or a reused entity index). The pre-fix negative `!ChannelTerminal` test
	/// fired `AppExit` here, tearing down every other session; every SSH client
	/// sends a closing ctrl+c, so concurrent sessions raced this constantly.
	#[beet_core::test]
	fn ctrl_c_on_bare_session_window_does_not_exit() {
		let app = ctrl_c_from(|world| world.spawn_empty().id());
		exited(&app).xpect_false();
	}
}
