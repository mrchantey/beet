use super::*;
use crate::style::ResolveStylesSet;
use crate::style::StylePlugin;
use beet_core::prelude::*;
#[allow(unused)]
use bevy::ecs::schedule::common_conditions;

#[derive(Default)]
pub struct CharcellPlugin;

impl Plugin for CharcellPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StylePlugin>()
			.configure_sets(
				PostUpdate,
				CharcellRenderSet.after(ResolveStylesSet),
			)
			.add_systems(
				PostUpdate,
				(
					prepare_charcell_tree,
					measure_nodes,
					layout_nodes,
					paint_nodes,
				)
					.chain()
					.in_set(CharcellRenderSet),
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
					.after(paint_nodes)
					.in_set(CharcellRenderSet),
			);
	}
}

/// PostUpdate set containing node layout, terminal render, and flush.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct CharcellRenderSet;
