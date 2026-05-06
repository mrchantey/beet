use super::apply_node_changes;
use beet_core::prelude::*;

pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			PostUpdate,
			apply_node_changes.in_set(ApplyNodeChanges),
		);
		#[cfg(feature = "termion")]
		{
			use crate::prelude::*;
			use bevy::ecs::schedule::common_conditions;
			use std::io::BufWriter;

			type R = termion::AsyncReader;
			type W = BufWriter<std::io::Stdout>;

			app.add_systems(
				PostUpdate,
				render_terminal::<R, W>.after(ApplyNodeChanges),
			);
			app.add_systems(
				PreUpdate,
				enable_raw_mode
					.run_if(common_conditions::resource_added::<RawMode>),
			);
			app.add_systems(
				PostUpdate,
				(
					flush_terminals::<R, W>.after(render_terminal::<R, W>),
					disable_raw_mode
						.run_if(common_conditions::resource_removed::<RawMode>),
					restore_terminals::<R, W>
						.run_if(common_conditions::on_message::<AppExit>),
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
