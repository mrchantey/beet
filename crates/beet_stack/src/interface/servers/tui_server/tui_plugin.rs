use crate::interface::visit_root;
use crate::prelude::widgets::handle_scroll_input;
use crate::stack::PropagateChanges;
use crate::stack::StackPlugin;
use beet_core::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use bevy_ratatui::RatatuiPlugins;


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
		.add_systems(PostStartup, visit_root);

		// Systems: handle input and draw, all after PropagateChanges
		app.add_systems(
			PostUpdate,
			(handle_scroll_input, super::draw_system)
				.chain()
				.after(PropagateChanges),
		)
		.add_systems(PostUpdate, exit_system);
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
