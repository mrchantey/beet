use beet_core::prelude::*;
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
		app.init_plugin::<StackPlugin>().add_plugins((
			MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
				Duration::from_secs_f32(1. / 60.),
			)),
			RatatuiPlugins {
				enable_kitty_protocol: true,
				enable_mouse_capture: true,
				enable_input_forwarding: true,
			},
		));
		// .add_systems(PreUpdate, super::tui_input::tui_input_system)
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
