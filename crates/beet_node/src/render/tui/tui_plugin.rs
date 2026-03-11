// use crate::prelude::widgets::handle_scroll_input;
use crate::prelude::*;
// use crate::stack::PropagateChanges;
// use crate::stack::StackPlugin;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiPlugins;



/// Top level Bevy plugin that sets up [`bevy_ratatui`], inserts TUI resources,
/// and registers the input/draw systems.
///
/// Only available on non-wasm targets since [`bevy_ratatui`] depends on
/// a terminal backend (crossterm).
///
/// Add this plugin alongside [`StackPlugin`] when building a TUI app.
/// All boilerplate for the terminal lifecycle is handled here.
///
/// # Includes
/// - [`StackPlugin`]
/// - [`MinimalPlugins`]
#[derive(Default)]
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
		.init_plugin::<InputPlugin>()
		.add_systems(PreUpdate, pointer_input_system)
		// .add_systems(
		// 	PostUpdate,
		// 	(handle_scroll_input, super::draw_system)
		// 		.chain()
		// 		.after(PropagateChanges),
		// )
		.add_systems(Startup, spawn_pointer)
		.add_systems(PostUpdate, exit_system);
	}
}

fn spawn_pointer(mut commands: Commands) { commands.spawn(PrimaryPointer); }
