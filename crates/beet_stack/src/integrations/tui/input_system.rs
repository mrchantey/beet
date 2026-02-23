use beet_core::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy_ratatui::RatatuiContext;
use bevy_ratatui::event::MouseMessage;
use ratatui::crossterm::event::MouseEvent;
use ratatui::crossterm::event::MouseEventKind;



pub fn mouse_input_system(
	mut messages: MessageReader<MouseMessage>,
	mut _commands: Commands,
	context: Res<RatatuiContext>,
) -> Result {
	for message in messages.read() {
		// println!("Mouse event: {:?}", message.0);
		// message.
		// let MouseEvent {
		// 	kind, column, row, ..
		// } = message.0;
		// let size = context.size()?;
		// let _column = column as f32 / size.width as f32;
		// let _row = row as f32 / size.height as f32;
		// match kind {
		// 	MouseEventKind::Down(_mouse_button) => {
		// 		todo!()
		// 	}
		// 	MouseEventKind::Up(_mouse_button) => {
		// 		todo!()
		// 	}
		// 	MouseEventKind::Drag(_mouse_button) => {
		// 		todo!()
		// 	}
		// 	MouseEventKind::Moved => todo!(),
		// 	MouseEventKind::ScrollDown => todo!(),
		// 	MouseEventKind::ScrollUp => todo!(),
		// 	MouseEventKind::ScrollLeft => todo!(),
		// 	MouseEventKind::ScrollRight => todo!(),
		// }
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
