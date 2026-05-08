use beet_core::prelude::*;
#[allow(unused)]
use bevy::ecs::schedule::common_conditions;

pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		use crate::prelude::*;

		// Node layout always runs in PostUpdate.
		app.add_systems(
			PostUpdate,
			apply_node_changes.in_set(CharcellRenderStep),
		);

		// Terminal-specific systems: input, render, flush.
		#[cfg(feature = "terminal")]
		app.add_observer(exit_ctrl_c)
			.add_systems(PreUpdate, terminal_events)
			.add_systems(
				PostUpdate,
				(
					render_terminal,
					flush_terminals,
					restore_terminals
						.run_if(common_conditions::on_message::<AppExit>),
				)
					.chain()
					.after(apply_node_changes)
					.in_set(CharcellRenderStep),
			);
	}
}

/// PostUpdate set containing node layout, terminal render, and flush.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct CharcellRenderStep;
