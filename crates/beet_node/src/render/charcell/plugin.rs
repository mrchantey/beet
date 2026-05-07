use super::apply_node_changes;
use beet_core::prelude::*;

pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			PostUpdate,
			apply_node_changes.in_set(ApplyNodeChanges),
		);
		#[cfg(feature = "terminal")]
		{
			use crate::prelude::*;
			use bevy::ecs::schedule::common_conditions;
			app.add_systems(PreUpdate, terminal_events).add_systems(
				PostUpdate,
				(
					render_terminal,
					flush_terminals,
					restore_terminals
						.run_if(common_conditions::on_message::<AppExit>),
				)
					.chain()
					.after(ApplyNodeChanges),
			);
		}
		#[cfg(feature = "tui")]
		{
			app.add_plugins(bevy_ratatui::RatatuiPlugins {
				enable_kitty_protocol: true,
				enable_mouse_capture: true,
				enable_input_forwarding: true,
			});
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ApplyNodeChanges;
