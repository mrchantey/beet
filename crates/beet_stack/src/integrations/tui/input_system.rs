//! TUI input handling: keyboard shortcuts and pointer-to-entity routing.
//!
//! The [`pointer_input_system`] reads crossterm mouse messages, resolves
//! the terminal cell position to an entity via [`TuiSpanMap`], and
//! triggers [`PointerDown`], [`PointerUp`], [`PointerOver`], and
//! [`PointerOut`] entity events as appropriate.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy_ratatui::event::MouseMessage;
use ratatui::crossterm::event::MouseEventKind;



pub fn pointer_input_system(
	mut messages: MessageReader<MouseMessage>,
	mut commands: Commands,
	span_map: Option<Res<TuiSpanMap>>,
	mut pointers: Query<(Entity, &mut Pointer), With<PrimaryPointer>>,
) -> Result {
	let Some(span_map) = span_map else {
		// No span map yet (nothing rendered), drain messages
		messages.clear();
		return Ok(());
	};

	let Ok((pointer_entity, mut pointer)) = pointers.single_mut() else {
		// No primary pointer spawned yet, drain messages
		messages.clear();
		return Ok(());
	};

	for message in messages.read() {
		let pos = TuiPos::new(message.0.row, message.0.column);
		let target = span_map.get(pos);

		match message.0.kind {
			MouseEventKind::Down(_) => {
				if let Some(entity) = target {
					commands.entity(entity).trigger_target(PointerDown {
						pointer: pointer_entity,
					});
				}
			}
			MouseEventKind::Up(_) => {
				if let Some(entity) = target {
					commands.entity(entity).trigger_target(PointerUp {
						pointer: pointer_entity,
					});
				}
			}
			MouseEventKind::Moved | MouseEventKind::Drag(_) => {
				let prev = pointer.hover;
				match (prev, target) {
					// Pointer moved from one entity to a different one
					(Some(old), Some(new)) if old != new => {
						commands.entity(old).trigger_target(PointerOut {
							pointer: pointer_entity,
						});
						commands.entity(new).trigger_target(PointerOver {
							pointer: pointer_entity,
						});
						pointer.hover = Some(new);
					}
					// Pointer entered an entity from empty space
					(None, Some(new)) => {
						commands.entity(new).trigger_target(PointerOver {
							pointer: pointer_entity,
						});
						pointer.hover = Some(new);
					}
					// Pointer left an entity into empty space
					(Some(old), None) => {
						commands.entity(old).trigger_target(PointerOut {
							pointer: pointer_entity,
						});
						pointer.hover = None;
					}
					// Same entity or still empty, nothing to do
					_ => {}
				}
			}
			// Scroll events are handled by the scrollbar widget
			MouseEventKind::ScrollDown
			| MouseEventKind::ScrollUp
			| MouseEventKind::ScrollLeft
			| MouseEventKind::ScrollRight => {}
		}
	}
	Ok(())
}




pub fn exit_system(
	mut messages: MessageReader<KeyboardInput>,
	mut commands: Commands,
) {
	use bevy::input::keyboard::Key;
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
