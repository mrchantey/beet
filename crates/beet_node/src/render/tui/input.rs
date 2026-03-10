use beet_core::prelude::*;
use bevy::input::keyboard::KeyboardInput;

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
