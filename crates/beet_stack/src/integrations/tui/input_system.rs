//! TUI input handling: keyboard shortcuts and mouse-to-entity routing.
//!
//! The [`mouse_input_system`] reads crossterm mouse messages, resolves
//! the terminal cell position to an entity via [`TuiSpanMap`], and
//! triggers [`TuiMouseDown`], [`TuiMouseUp`], [`TuiMouseOver`], and
//! [`TuiMouseOut`] entity events as appropriate.
use beet_core::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy_ratatui::event::MouseMessage;
use ratatui::crossterm::event::MouseEventKind;

use super::TuiHoverState;
use super::TuiMouseDown;
use super::TuiMouseOut;
use super::TuiMouseOver;
use super::TuiMouseUp;
use super::TuiSpanMap;



pub fn mouse_input_system(
	mut messages: MessageReader<MouseMessage>,
	mut commands: Commands,
	span_map: Option<Res<TuiSpanMap>>,
	mut hover_state: ResMut<TuiHoverState>,
) -> Result {
	let Some(span_map) = span_map else {
		// No span map yet (nothing rendered), drain messages
		for _message in messages.read() {}
		return Ok(());
	};

	for message in messages.read() {
		let col = message.0.column;
		let row = message.0.row;
		let target = span_map.get(col, row);

		match message.0.kind {
			MouseEventKind::Down(_) => {
				if let Some(entity) = target {
					commands.trigger(TuiMouseDown(entity));
				}
			}
			MouseEventKind::Up(_) => {
				if let Some(entity) = target {
					commands.trigger(TuiMouseUp(entity));
				}
			}
			MouseEventKind::Moved | MouseEventKind::Drag(_) => {
				let prev = hover_state.hovered;
				match (prev, target) {
					// Cursor moved from one entity to a different one
					(Some(old), Some(new)) if old != new => {
						commands.trigger(TuiMouseOut(old));
						commands.trigger(TuiMouseOver(new));
						hover_state.hovered = Some(new);
					}
					// Cursor entered an entity from empty space
					(None, Some(new)) => {
						commands.trigger(TuiMouseOver(new));
						hover_state.hovered = Some(new);
					}
					// Cursor left an entity into empty space
					(Some(old), None) => {
						commands.trigger(TuiMouseOut(old));
						hover_state.hovered = None;
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
