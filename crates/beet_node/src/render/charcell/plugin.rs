use beet_core::prelude::*;
use bevy::ecs::schedule::common_conditions;

pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		use crate::prelude::*;
		app.add_observer(exit_ctrl_c)
			.add_systems(PreUpdate, terminal_events)
			.add_systems(
				PostUpdate,
				(
					apply_node_changes,
					render_terminal,
					flush_terminals,
					restore_terminals
						.run_if(common_conditions::on_message::<AppExit>),
				)
					.chain()
					.in_set(CharcellRenderStep),
			);
		// #[cfg(feature = "tui")]
		// {
		// 	app.add_plugins(bevy_ratatui::RatatuiPlugins {
		// 		enable_kitty_protocol: true,
		// 		enable_mouse_capture: true,
		// 		enable_input_forwarding: true,
		// 	});
		// }
	}
}

/// A PostUpdate step writing and flushing the terminals
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct CharcellRenderStep;
