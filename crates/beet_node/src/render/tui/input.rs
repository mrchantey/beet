use crate::prelude::*;
use beet_core::prelude::*;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy_ratatui::event::KeyMessage;
use bevy_ratatui::event::MouseMessage;
use ratatui::crossterm::event::MouseEventKind;

pub fn pointer_input_system(
	mut messages: MessageReader<MouseMessage>,
	mut commands: Commands,
	span_maps: Query<&TuiSpanMap>,
	mut pointers: Query<(Entity, &mut Pointer), With<PrimaryPointer>>,
) -> Result {
	let Ok((pointer_entity, mut pointer)) = pointers.single_mut() else {
		// No primary pointer spawned yet, drain messages
		messages.clear();
		return Ok(());
	};

	for message in messages.read() {
		let pos = TuiPos::new(message.0.row, message.0.column);
		for span_map in span_maps.iter() {
			let target = span_map.get(pos);

			match message.0.kind {
				MouseEventKind::Down(_) => {
					if let Some(entity) = target {
						commands
							.entity(entity)
							.trigger(PointerDown::new(pointer_entity));
					}
				}
				MouseEventKind::Up(_) => {
					if let Some(entity) = target {
						commands
							.entity(entity)
							.trigger(PointerUp::new(pointer_entity));
					}
				}
				MouseEventKind::Moved | MouseEventKind::Drag(_) => {
					let prev = pointer.hover;
					match (prev, target) {
						// Pointer moved from one entity to a different one
						(Some(old), Some(new)) if old != new => {
							commands
								.entity(old)
								.try_trigger(PointerOut::new(pointer_entity));
							commands
								.entity(new)
								.trigger(PointerOver::new(pointer_entity));
							pointer.hover = Some(new);
						}
						// Pointer entered an entity from empty space
						(None, Some(new)) => {
							commands
								.entity(new)
								.trigger(PointerOver::new(pointer_entity));
							pointer.hover = Some(new);
						}
						// Pointer left an entity into empty space
						(Some(old), None) => {
							commands
								.entity(old)
								.trigger(PointerOut::new(pointer_entity));
							pointer.hover = None;
						}
						// Same entity or still empty, nothing to do
						_ => {}
					}
				}
				// Scroll events are handled by scroll_input_system
				MouseEventKind::ScrollDown
				| MouseEventKind::ScrollUp
				| MouseEventKind::ScrollLeft
				| MouseEventKind::ScrollRight => {}
			}
		}
	}
	Ok(())
}



/// Updates [`TuiScrollState`] from keyboard arrow keys and mouse
/// scroll wheel events.
///
/// Reads both [`KeyboardInput`] and [`MouseMessage`] messages each
/// frame and adjusts the scroll offset on any entity that carries a
/// [`TuiScrollState`] component.
pub fn scroll_input_system(
	mut key_messages: MessageReader<KeyMessage>,
	mut mouse_messages: MessageReader<MouseMessage>,
	mut scroll_query: Query<(&mut TuiWidget, &mut TuiScrollState)>,
) {
	const SCROLL_LINES: u16 = 1;
	const MOUSE_SCROLL_LINES: u16 = 3;

	let mut delta: i32 = 0;

	for message in key_messages.read().filter(|msg| msg.is_press()) {
		use ratatui::crossterm::event::KeyCode;
		match message.code {
			KeyCode::Down => {
				delta += SCROLL_LINES as i32;
			}
			KeyCode::Up => {
				delta -= SCROLL_LINES as i32;
			}
			_ => {}
		}
	}

	for message in mouse_messages.read() {
		match message.0.kind {
			MouseEventKind::ScrollDown => {
				delta += MOUSE_SCROLL_LINES as i32;
			}
			MouseEventKind::ScrollUp => {
				delta -= MOUSE_SCROLL_LINES as i32;
			}
			_ => {}
		}
	}

	if delta == 0 {
		return;
	}

	for (mut widget, mut scroll) in scroll_query.iter_mut() {
		widget.set_changed();
		if delta > 0 {
			scroll.scroll_down(delta as u16);
		} else {
			scroll.scroll_up(delta.unsigned_abs() as u16);
		}
	}
}

pub fn exit_system(
	mut messages: MessageReader<KeyboardInput>,
	mut commands: Commands,
) {
	for message in messages.read() {
		match &message.logical_key {
			Key::Character(val) if val == "q" => {
				// TODO allow textbox input
				commands.write_message(AppExit::Success);
			}
			Key::Escape => {
				commands.write_message(AppExit::Success);
			}
			_ => {}
		}
	}
}
