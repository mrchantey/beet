use beet_core::prelude::*;
use bevy::input::ButtonState;
use bevy::input::keyboard::NativeKeyCode;
use bevy::prelude::KeyCode;
use bevy::prelude::MouseButton;
use termion::event::Event as TermionEvent;

use crate::render::Terminal;




/// Input event from the terminal, targeted at the terminal entity.
#[derive(Debug, Clone, EntityTargetEvent)]
#[event(auto_propagate)]
pub enum TerminalEvent {
	Key(KeyPress),
	Mouse(MouseEvent),
	Unsupported(Vec<u8>),
}

pub fn terminal_events(
	mut commands: Commands,
	mut query: Populated<(Entity, &mut Terminal)>,
) -> Result {
	for (entity, mut terminal) in query.iter_mut() {
		for event in terminal.read_events()? {
			let event: TerminalEvent = event.into();
			commands.entity(entity).trigger_target(event);
		}
	}

	Ok(())
}

impl From<TermionEvent> for TerminalEvent {
	fn from(value: TermionEvent) -> Self {
		match value {
			TermionEvent::Key(key) => Self::Key(termion_to_key(key)),
			TermionEvent::Mouse(mouse_event) => {
				Self::Mouse(termion_to_mouse(mouse_event))
			}
			TermionEvent::Unsupported(items) => Self::Unsupported(items),
		}
	}
}

#[derive(Debug, Copy, Clone, Get)]
pub struct MouseEvent {
	/// Zero-indexed charcell coordinates
	position: UVec2,
	button: MouseButton,
	state: ButtonState,
}

#[rustfmt::skip]
fn termion_to_mouse(mouse_event: termion::event::MouseEvent) -> MouseEvent {
	use termion::event::MouseEvent::*;
	match mouse_event {
		Press(button, x, y) => MouseEvent {
			position: UVec2::new(x as u32 - 1, y as u32 - 1),
			button: match button {
				termion::event::MouseButton::Left => MouseButton::Left,
				termion::event::MouseButton::Right => MouseButton::Right,
				termion::event::MouseButton::Middle => MouseButton::Middle,
				termion::event::MouseButton::WheelUp => MouseButton::Other(0),
				termion::event::MouseButton::WheelDown => MouseButton::Other(1),
				termion::event::MouseButton::WheelLeft => MouseButton::Other(2),
				termion::event::MouseButton::WheelRight => MouseButton::Other(3),
			},
			state: ButtonState::Pressed,
		},
		Release(x, y) => MouseEvent {
			position: UVec2::new(x as u32 - 1, y as u32 - 1),
			button: MouseButton::Left, // TODO: Termion doesn't specify which button was released
			state: ButtonState::Released,
		},
		Hold(x, y) => MouseEvent {
			position: UVec2::new(x as u32 - 1, y as u32 - 1),
			button: MouseButton::Left, // TODO: Termion doesn't specify which button is being held
			state: ButtonState::Pressed,
		},
	}
}

bitflags::bitflags! {
	#[derive(Debug, Copy, Clone,PartialEq, Eq, PartialOrd, Ord)]
	pub struct KeyModifier: u8 {
  const CTRL = 1 << 0;
  const ALT = 1 << 1;
  const SHIFT = 1 << 2;
 }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Get)]
pub struct KeyPress {
	key: KeyCode,
	modifier: KeyModifier,
}
impl KeyPress {
	pub const CTRL_C: Self = Self {
		key: KeyCode::KeyC,
		modifier: KeyModifier::CTRL,
	};

	pub fn with_modifier(mut self, modifier: KeyModifier) -> Self {
		self.modifier |= modifier;
		self
	}
}
impl KeyPress {
	pub fn new(key: KeyCode, modifier: KeyModifier) -> Self {
		Self { key, modifier }
	}
	pub fn unmodified(key: KeyCode) -> Self {
		Self {
			key,
			modifier: KeyModifier::empty(),
		}
	}
}

#[rustfmt::skip]
fn termion_to_key(key: termion::event::Key) -> KeyPress {
	use termion::event::Key::*;
	match key {
		Backspace => KeyPress::unmodified(KeyCode::Backspace),
		Left => KeyPress::unmodified(KeyCode::ArrowLeft),
		ShiftLeft => KeyPress::new(KeyCode::ShiftLeft, KeyModifier::SHIFT),
		AltLeft => KeyPress::new(KeyCode::AltLeft, KeyModifier::ALT),
		CtrlLeft => KeyPress::new(KeyCode::ControlLeft, KeyModifier::CTRL),
		Right => KeyPress::unmodified(KeyCode::ArrowRight),
		ShiftRight => KeyPress::new(KeyCode::ShiftRight, KeyModifier::SHIFT),
		AltRight => KeyPress::new(KeyCode::AltRight, KeyModifier::ALT),
		CtrlRight => KeyPress::new(KeyCode::ControlRight, KeyModifier::CTRL),
		Up => KeyPress::unmodified(KeyCode::ArrowUp),
		ShiftUp => KeyPress::new(KeyCode::ArrowUp, KeyModifier::SHIFT),
		AltUp => KeyPress::new(KeyCode::ArrowUp, KeyModifier::ALT),
		CtrlUp => KeyPress::new(KeyCode::ArrowUp, KeyModifier::CTRL),
		Down => KeyPress::unmodified(KeyCode::ArrowDown),
		ShiftDown => KeyPress::new(KeyCode::ArrowDown, KeyModifier::SHIFT),
		AltDown => KeyPress::new(KeyCode::ArrowDown, KeyModifier::ALT),
		CtrlDown => KeyPress::new(KeyCode::ArrowDown, KeyModifier::CTRL),
		Home => KeyPress::unmodified(KeyCode::Home),
		CtrlHome => KeyPress::new(KeyCode::Home, KeyModifier::CTRL),
		End => KeyPress::unmodified(KeyCode::End),
		CtrlEnd => KeyPress::new(KeyCode::End, KeyModifier::CTRL),
		PageUp => KeyPress::unmodified(KeyCode::PageUp),
		PageDown => KeyPress::unmodified(KeyCode::PageDown),
		BackTab => KeyPress::new(KeyCode::Tab, KeyModifier::SHIFT),
		Delete => KeyPress::unmodified(KeyCode::Delete),
		Insert => KeyPress::unmodified(KeyCode::Insert),
		F(n) => match n {
			1 => KeyPress::unmodified(KeyCode::F1),
			2 => KeyPress::unmodified(KeyCode::F2),
			3 => KeyPress::unmodified(KeyCode::F3),
			4 => KeyPress::unmodified(KeyCode::F4),
			5 => KeyPress::unmodified(KeyCode::F5),
			6 => KeyPress::unmodified(KeyCode::F6),
			7 => KeyPress::unmodified(KeyCode::F7),
			8 => KeyPress::unmodified(KeyCode::F8),
			9 => KeyPress::unmodified(KeyCode::F9),
			10 => KeyPress::unmodified(KeyCode::F10),
			11 => KeyPress::unmodified(KeyCode::F11),
			12 => KeyPress::unmodified(KeyCode::F12),
			13 => KeyPress::unmodified(KeyCode::F13),
			14 => KeyPress::unmodified(KeyCode::F14),
			15 => KeyPress::unmodified(KeyCode::F15),
			16 => KeyPress::unmodified(KeyCode::F16),
			17 => KeyPress::unmodified(KeyCode::F17),
			18 => KeyPress::unmodified(KeyCode::F18),
			19 => KeyPress::unmodified(KeyCode::F19),
			20 => KeyPress::unmodified(KeyCode::F20),
			21 => KeyPress::unmodified(KeyCode::F21),
			22 => KeyPress::unmodified(KeyCode::F22),
			23 => KeyPress::unmodified(KeyCode::F23),
			24 => KeyPress::unmodified(KeyCode::F24),
			25 => KeyPress::unmodified(KeyCode::F25),
			26 => KeyPress::unmodified(KeyCode::F26),
			27 => KeyPress::unmodified(KeyCode::F27),
			28 => KeyPress::unmodified(KeyCode::F28),
			29 => KeyPress::unmodified(KeyCode::F29),
			30 => KeyPress::unmodified(KeyCode::F30),
			31 => KeyPress::unmodified(KeyCode::F31),
			32 => KeyPress::unmodified(KeyCode::F32),
			33 => KeyPress::unmodified(KeyCode::F33),
			34 => KeyPress::unmodified(KeyCode::F34),
			35 => KeyPress::unmodified(KeyCode::F35),
			_ => KeyPress::unmodified(KeyCode::Unidentified(NativeKeyCode::Unidentified)),
		},

		Char(c) => KeyPress::unmodified(char_to_keycode(c)),
		Alt(c) => KeyPress::new(char_to_keycode(c), KeyModifier::ALT),
		Ctrl(c) => KeyPress::new(char_to_keycode(c), KeyModifier::CTRL),
		Null => KeyPress::unmodified(KeyCode::Unidentified(NativeKeyCode::Unidentified)),
		Esc => KeyPress::unmodified(KeyCode::Escape),
		_ => KeyPress::unmodified(KeyCode::Unidentified(NativeKeyCode::Unidentified)),
	}
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
		'\n' => KeyCode::Enter,
		'\t' => KeyCode::Tab,
		_ => KeyCode::Unidentified(NativeKeyCode::Unidentified),
	}
}
