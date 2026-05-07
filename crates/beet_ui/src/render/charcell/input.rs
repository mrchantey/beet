use crate::render::Terminal;
use beet_core::prelude::*;
use bevy::input::keyboard::NativeKeyCode;
use bevy::prelude::KeyCode;
use bevy::prelude::MouseButton;
use vte::Params;
use vte::Perform;

// ── InputParser ───────────────────────────────────────────────────────────────

/// Stateful terminal input parser built on [`vte::Parser`].
///
/// The parser is stateful across calls so partial escape sequences
/// spanning multiple reads are handled correctly.
pub struct InputParser {
	parser: vte::Parser,
	performer: Performer,
}

impl Default for InputParser {
	fn default() -> Self {
		Self {
			parser: vte::Parser::new(),
			performer: Performer::default(),
		}
	}
}

impl InputParser {
	pub fn new() -> Self { Self::default() }

	/// Parse a byte slice and return all complete [`TerminalEvent`]s.
	pub fn parse(&mut self, bytes: &[u8]) -> Result<Vec<TerminalEvent>> {
		self.parser.advance(&mut self.performer, bytes);
		Ok(core::mem::take(&mut self.performer.events))
	}
}

// ── Bevy systems ──────────────────────────────────────────────────────────────

/// Read input events from each [`Terminal`] and forward them as ECS events.
pub fn terminal_events(
	mut commands: Commands,
	mut query: Populated<(Entity, &mut Terminal)>,
) -> Result {
	for (entity, mut terminal) in query.iter_mut() {
		for event in terminal.read_events()? {
			commands.entity(entity).trigger_target(event);
		}
	}
	Ok(())
}


// ── Public types ──────────────────────────────────────────────────────────────

/// Input event from the terminal, targeted at the terminal entity.
#[derive(Debug, Clone, EntityTargetEvent)]
#[event(auto_propagate)]
pub enum TerminalEvent {
	Key(KeyPress),
	Mouse(MouseEvent),
	/// A bracketed paste payload, requires [`TerminalConfig::bracketed_paste`] to be enabled.
	Paste(String),
	/// Terminal was resized, ie via `SIGWINCH`.
	Resize(UVec2),
	Unsupported(Vec<u8>),
}

/// A mouse input event.
#[derive(Debug, Copy, Clone)]
pub struct MouseEvent {
	/// Zero-indexed charcell coordinates.
	pub position: UVec2,
	/// The kind of event.
	pub kind: MouseEventKind,
}

/// Kind of mouse event.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MouseEventKind {
	/// A button was pressed.
	Press(MouseButton),
	/// A button was released.
	Release(MouseButton),
	/// Mouse moved without any button held (any-motion tracking).
	Move,
	/// Mouse moved while a button was held (drag).
	Drag(MouseButton),
	/// Scroll wheel event.
	Scroll(ScrollDirection),
}

/// Scroll wheel direction.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScrollDirection {
	Up,
	Down,
	Left,
	Right,
}

bitflags::bitflags! {
	/// Keyboard modifier keys.
	#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
	pub struct KeyModifier: u8 {
		const CTRL  = 1 << 0;
		const ALT   = 1 << 1;
		const SHIFT = 1 << 2;
	}
}

/// A keyboard press event.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Get)]
pub struct KeyPress {
	key: KeyCode,
	modifier: KeyModifier,
	/// Printed character for this key, ie `Some('a')` for the A key, `None` for F1.
	pub char: Option<char>,
}

impl KeyPress {
	/// Ctrl+C shortcut constant.
	pub const CTRL_C: Self = Self {
		key: KeyCode::KeyC,
		modifier: KeyModifier::CTRL,
		char: None,
	};

	/// Create a key press with a modifier.
	pub fn new(key: KeyCode, modifier: KeyModifier) -> Self {
		Self {
			key,
			modifier,
			char: None,
		}
	}

	/// Create a key press with no modifier.
	pub fn unmodified(key: KeyCode) -> Self {
		Self {
			key,
			modifier: KeyModifier::empty(),
			char: None,
		}
	}

	/// Add an additional modifier, preserving any already set.
	pub fn with_modifier(mut self, modifier: KeyModifier) -> Self {
		self.modifier |= modifier;
		self
	}

	/// Create a key press with full field control.
	pub fn with_char(
		key: KeyCode,
		modifier: KeyModifier,
		char: Option<char>,
	) -> Self {
		Self {
			key,
			modifier,
			char,
		}
	}
}

// ── VTE performer ─────────────────────────────────────────────────────────────

/// Stateful VTE performer that accumulates [`TerminalEvent`]s.
#[derive(Default)]
struct Performer {
	events: Vec<TerminalEvent>,
	/// Set when `ESC O` is received; the next `print` completes an SS3 sequence.
	ss3_pending: bool,
	/// Set during a bracketed paste sequence (`CSI 200~` … `CSI 201~`).
	in_paste: bool,
	/// Accumulator for bracketed paste content.
	paste_buf: String,
}

impl Performer {
	fn push_key(&mut self, key: KeyPress) {
		self.events.push(TerminalEvent::Key(key));
	}

	fn push_mouse(&mut self, event: MouseEvent) {
		self.events.push(TerminalEvent::Mouse(event));
	}

	/// Parse an SGR mouse event: `ESC [ < cb ; cx ; cy M/m`.
	///
	/// `released` is true when the final character is `m`, false for `M`.
	fn parse_sgr_mouse(&mut self, cb: u16, cx: u16, cy: u16, released: bool) {
		// Convert from 1-indexed to 0-indexed coordinates.
		let position = UVec2::new(
			cx.saturating_sub(1) as u32,
			cy.saturating_sub(1) as u32,
		);

		// Strip modifier bits (Shift=4, Alt=8, Ctrl=16) to get the base code.
		let base = cb & !(4 | 8 | 16);

		let kind = if released {
			let button = match base {
				0 => MouseButton::Left,
				1 => MouseButton::Middle,
				2 => MouseButton::Right,
				_ => return,
			};
			MouseEventKind::Release(button)
		} else {
			match base {
				64 => MouseEventKind::Scroll(ScrollDirection::Up),
				65 => MouseEventKind::Scroll(ScrollDirection::Down),
				66 => MouseEventKind::Scroll(ScrollDirection::Left),
				67 => MouseEventKind::Scroll(ScrollDirection::Right),
				// Any-motion without button held
				35 => MouseEventKind::Move,
				// Drag (motion with button held)
				32 => MouseEventKind::Drag(MouseButton::Left),
				33 => MouseEventKind::Drag(MouseButton::Middle),
				34 => MouseEventKind::Drag(MouseButton::Right),
				// Button press
				0 => MouseEventKind::Press(MouseButton::Left),
				1 => MouseEventKind::Press(MouseButton::Middle),
				2 => MouseEventKind::Press(MouseButton::Right),
				_ => return,
			}
		};

		self.push_mouse(MouseEvent { position, kind });
	}
}

impl Perform for Performer {
	/// Handle printable characters and SS3 follow-up bytes.
	fn print(&mut self, c: char) {
		// Accumulate paste content when inside a bracketed paste sequence.
		if self.in_paste {
			self.paste_buf.push(c);
			return;
		}

		// Complete a pending SS3 sequence (ESC O ...) for F1-F4 and cursor keys.
		if self.ss3_pending {
			self.ss3_pending = false;
			let key = match c {
				'P' => KeyPress::unmodified(KeyCode::F1),
				'Q' => KeyPress::unmodified(KeyCode::F2),
				'R' => KeyPress::unmodified(KeyCode::F3),
				'S' => KeyPress::unmodified(KeyCode::F4),
				'H' => KeyPress::unmodified(KeyCode::Home),
				'F' => KeyPress::unmodified(KeyCode::End),
				'A' => KeyPress::unmodified(KeyCode::ArrowUp),
				'B' => KeyPress::unmodified(KeyCode::ArrowDown),
				'C' => KeyPress::unmodified(KeyCode::ArrowRight),
				'D' => KeyPress::unmodified(KeyCode::ArrowLeft),
				_ => {
					self.events.push(TerminalEvent::Unsupported(vec![
						0x1b, b'O', c as u8,
					]));
					return;
				}
			};
			self.push_key(key);
			return;
		}

		// DEL byte (0x7F) — sent by Backspace in most terminals.
		if c == '\x7f' {
			self.push_key(KeyPress::unmodified(KeyCode::Backspace));
			return;
		}

		// Regular printable character.
		let (key, modifier) = char_to_key(c);
		self.push_key(KeyPress::with_char(key, modifier, Some(c)));
	}

	/// Handle C0 control characters (0x00–0x1F).
	fn execute(&mut self, byte: u8) {
		// Include newlines inside paste content.
		if self.in_paste {
			match byte {
				0x0A | 0x0D => self.paste_buf.push('\n'),
				_ => {}
			}
			return;
		}

		let key = match byte {
			0x08 => KeyPress::unmodified(KeyCode::Backspace),
			0x09 => KeyPress::with_char(
				KeyCode::Tab,
				KeyModifier::empty(),
				Some('\t'),
			),
			0x0A | 0x0D => KeyPress::with_char(
				KeyCode::Enter,
				KeyModifier::empty(),
				Some('\n'),
			),
			// Ctrl+A through Ctrl+Z (0x01–0x1A)
			c @ 0x01..=0x1A => {
				let ch = (c - 0x01 + b'a') as char;
				let (key, _) = char_to_key(ch);
				KeyPress::new(key, KeyModifier::CTRL)
			}
			// Ctrl+\ through Ctrl+_ (0x1C–0x1F)
			c @ 0x1C..=0x1F => {
				let ch = (c - 0x1C + b'4') as char;
				let (key, _) = char_to_key(ch);
				KeyPress::new(key, KeyModifier::CTRL)
			}
			_ => return,
		};
		self.push_key(key);
	}

	/// Handle CSI sequences: arrow keys, F-keys, mouse, special keys.
	fn csi_dispatch(
		&mut self,
		params: &Params,
		intermediates: &[u8],
		_ignore: bool,
		c: char,
	) {
		// Collect first sub-param from each `;`-separated group.
		let p: Vec<u16> = params
			.iter()
			.map(|sub| sub.iter().next().copied().unwrap_or(0))
			.collect();

		match (intermediates, c) {
			// ── SGR mouse: ESC [ < cb ; cx ; cy M/m ──────────────────────────
			(b"<", 'M') | (b"<", 'm') if p.len() >= 3 => {
				self.parse_sgr_mouse(p[0], p[1], p[2], c == 'm');
			}

			// ── Arrow keys (no modifier) ──────────────────────────────────────
			(b"", 'A') if p.len() <= 1 => {
				self.push_key(KeyPress::unmodified(KeyCode::ArrowUp))
			}
			(b"", 'B') if p.len() <= 1 => {
				self.push_key(KeyPress::unmodified(KeyCode::ArrowDown))
			}
			(b"", 'C') if p.len() <= 1 => {
				self.push_key(KeyPress::unmodified(KeyCode::ArrowRight))
			}
			(b"", 'D') if p.len() <= 1 => {
				self.push_key(KeyPress::unmodified(KeyCode::ArrowLeft))
			}
			(b"", 'H') if p.is_empty() => {
				self.push_key(KeyPress::unmodified(KeyCode::Home))
			}
			(b"", 'F') if p.is_empty() => {
				self.push_key(KeyPress::unmodified(KeyCode::End))
			}

			// ── Arrow/nav keys with modifier: ESC [ 1 ; N X ──────────────────
			(b"", dir @ ('A' | 'B' | 'C' | 'D' | 'H' | 'F'))
				if p.len() == 2 =>
			{
				let modifier = decode_modifier(p[1]);
				let key = match dir {
					'A' => KeyCode::ArrowUp,
					'B' => KeyCode::ArrowDown,
					'C' => KeyCode::ArrowRight,
					'D' => KeyCode::ArrowLeft,
					'H' => KeyCode::Home,
					'F' => KeyCode::End,
					_ => unreachable!(),
				};
				self.push_key(KeyPress::new(key, modifier));
			}

			// ── Backward tab ─────────────────────────────────────────────────
			(b"", 'Z') => {
				self.push_key(KeyPress::new(KeyCode::Tab, KeyModifier::SHIFT))
			}

			// ── Bracketed paste: CSI 200~ start, CSI 201~ end ────────────────────
			(b"", '~') if p.first() == Some(&200) => {
				self.in_paste = true;
				self.paste_buf.clear();
			}
			(b"", '~') if p.first() == Some(&201) => {
				self.in_paste = false;
				self.events.push(TerminalEvent::Paste(core::mem::take(
					&mut self.paste_buf,
				)));
			}

			// ── Tilde-terminated special keys and F-keys ──────────────────────────
			(b"", '~') if !p.is_empty() => {
				let key = match (p[0], p.get(1).copied().unwrap_or(0)) {
					(1 | 7, _) => KeyPress::unmodified(KeyCode::Home),
					(2, _) => KeyPress::unmodified(KeyCode::Insert),
					(3, _) => KeyPress::unmodified(KeyCode::Delete),
					(4 | 8, _) => KeyPress::unmodified(KeyCode::End),
					(5, _) => KeyPress::unmodified(KeyCode::PageUp),
					(6, _) => KeyPress::unmodified(KeyCode::PageDown),
					(11, _) => KeyPress::unmodified(KeyCode::F1),
					(12, _) => KeyPress::unmodified(KeyCode::F2),
					(13, _) => KeyPress::unmodified(KeyCode::F3),
					(14, _) => KeyPress::unmodified(KeyCode::F4),
					(15, _) => KeyPress::unmodified(KeyCode::F5),
					(17, _) => KeyPress::unmodified(KeyCode::F6),
					(18, _) => KeyPress::unmodified(KeyCode::F7),
					(19, _) => KeyPress::unmodified(KeyCode::F8),
					(20, _) => KeyPress::unmodified(KeyCode::F9),
					(21, _) => KeyPress::unmodified(KeyCode::F10),
					(23, _) => KeyPress::unmodified(KeyCode::F11),
					(24, _) => KeyPress::unmodified(KeyCode::F12),
					_ => return,
				};
				self.push_key(key);
			}

			_ => {} // Unknown CSI — ignore silently
		}
	}

	/// Handle ESC sequences: Alt-key combos and SS3 prefix (F1-F4, cursor).
	fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
		if !intermediates.is_empty() {
			return;
		}
		match byte {
			// SS3 prefix — the next `print` call completes the sequence.
			b'O' => self.ss3_pending = true,
			// Alt+char: ESC followed by a printable byte.
			c if c.is_ascii_graphic() || c == b' ' => {
				let ch = c as char;
				let (key, _) = char_to_key(ch);
				self.push_key(KeyPress::with_char(
					key,
					KeyModifier::ALT,
					Some(ch),
				));
			}
			_ => {}
		}
	}
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Map a CSI modifier number to [`KeyModifier`] bitflags.
///
/// The encoding is `N - 1` where bit 0 = Shift, bit 1 = Alt, bit 2 = Ctrl.
fn decode_modifier(code: u16) -> KeyModifier {
	let bits = code.saturating_sub(1);
	let mut m = KeyModifier::empty();
	if bits & 1 != 0 {
		m |= KeyModifier::SHIFT;
	}
	if bits & 2 != 0 {
		m |= KeyModifier::ALT;
	}
	if bits & 4 != 0 {
		m |= KeyModifier::CTRL;
	}
	m
}

/// Map a `char` to a [`KeyCode`] and infer a Shift modifier for uppercase.
fn char_to_key(c: char) -> (KeyCode, KeyModifier) {
	let modifier = if c.is_ascii_uppercase() {
		KeyModifier::SHIFT
	} else {
		KeyModifier::empty()
	};
	let key = char_to_keycode(c.to_ascii_lowercase());
	(key, modifier)
}

fn char_to_keycode(c: char) -> KeyCode {
	match c {
		'a' => KeyCode::KeyA,
		'b' => KeyCode::KeyB,
		'c' => KeyCode::KeyC,
		'd' => KeyCode::KeyD,
		'e' => KeyCode::KeyE,
		'f' => KeyCode::KeyF,
		'g' => KeyCode::KeyG,
		'h' => KeyCode::KeyH,
		'i' => KeyCode::KeyI,
		'j' => KeyCode::KeyJ,
		'k' => KeyCode::KeyK,
		'l' => KeyCode::KeyL,
		'm' => KeyCode::KeyM,
		'n' => KeyCode::KeyN,
		'o' => KeyCode::KeyO,
		'p' => KeyCode::KeyP,
		'q' => KeyCode::KeyQ,
		'r' => KeyCode::KeyR,
		's' => KeyCode::KeyS,
		't' => KeyCode::KeyT,
		'u' => KeyCode::KeyU,
		'v' => KeyCode::KeyV,
		'w' => KeyCode::KeyW,
		'x' => KeyCode::KeyX,
		'y' => KeyCode::KeyY,
		'z' => KeyCode::KeyZ,
		'0' => KeyCode::Digit0,
		'1' => KeyCode::Digit1,
		'2' => KeyCode::Digit2,
		'3' => KeyCode::Digit3,
		'4' => KeyCode::Digit4,
		'5' => KeyCode::Digit5,
		'6' => KeyCode::Digit6,
		'7' => KeyCode::Digit7,
		'8' => KeyCode::Digit8,
		'9' => KeyCode::Digit9,
		' ' => KeyCode::Space,
		'\n' | '\r' => KeyCode::Enter,
		'\t' => KeyCode::Tab,
		'-' => KeyCode::Minus,
		'=' => KeyCode::Equal,
		'[' => KeyCode::BracketLeft,
		']' => KeyCode::BracketRight,
		'\\' => KeyCode::Backslash,
		';' => KeyCode::Semicolon,
		'\'' => KeyCode::Quote,
		',' => KeyCode::Comma,
		'.' => KeyCode::Period,
		'/' => KeyCode::Slash,
		'`' => KeyCode::Backquote,
		_ => KeyCode::Unidentified(NativeKeyCode::Unidentified),
	}
}
