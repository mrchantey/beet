use beet_core::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy_ratatui::RatatuiPlugins;

use crate::stack::StackPlugin;


/// Top level Bevy plugin that sets up [`bevy_ratatui`], inserts TUI resources,
/// and registers the input/draw systems.
///
/// Add this plugin alongside [`StackPlugin`] when building a TUI app.
/// All boilerplate for the terminal lifecycle is handled here.
///
/// # Includes
/// - [`StackPlugin`]
/// - [`MinimalPlugins`]
pub struct TuiPlugin;

impl Plugin for TuiPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
				Duration::from_secs_f32(1. / 60.),
			)),
			RatatuiPlugins {
				enable_kitty_protocol: true,
				enable_mouse_capture: true,
				enable_input_forwarding: true,
			},
		))
		.init_plugin::<StackPlugin>()
		.add_systems(PostUpdate, (super::draw_system, exit_system));
		// .add_systems(
		// 	Update,
		// 	(
		// 		super::tui_draw::tui_poll_responses,
		// 		super::tui_draw::tui_sync_tree,
		// 		super::tui_draw::tui_draw_system,
		// 	)
		// 		.chain(),
		// );
	}
}



fn exit_system(
	mut messages: MessageReader<KeyboardInput>,
	mut commands: Commands,
) {
	use bevy::input::keyboard::Key;
	for message in messages.read() {
		match &message.logical_key {
			Key::Character(val) if val == "q" => {
				commands.write_message(AppExit::Success);
			}
			Key::Escape => {
				commands.write_message(AppExit::Success);
			}
			_ => {}
		}
	}
}
