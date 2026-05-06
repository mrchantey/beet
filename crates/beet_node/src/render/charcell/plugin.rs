use super::apply_node_changes;
use beet_core::prelude::*;

pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			PostUpdate,
			apply_node_changes.in_set(ApplyNodeChanges),
		);
		#[cfg(feature = "termwiz")]
		{
			use crate::prelude::*;
			use bevy::ecs::schedule::common_conditions;
			app.add_systems(
				PostUpdate,
				render_terminal.after(ApplyNodeChanges),
			);
			app.add_systems(
				PreUpdate,
				enable_raw_mode
					.run_if(common_conditions::resource_added::<RawMode>),
			);
			app.add_systems(
				PostUpdate,
				(
					flush_stdio_terminals.after(render_terminal),
					disable_raw_mode
						.run_if(common_conditions::resource_removed::<RawMode>),
					try_disable_raw_mode
						.run_if(common_conditions::on_message::<AppExit>),
				)
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
