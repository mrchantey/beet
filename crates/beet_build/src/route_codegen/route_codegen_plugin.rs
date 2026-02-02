//! Route code generation plugin and schedule.
//!
//! This module provides the [`RouteCodegenPlugin`] that registers all systems
//! for generating route code from source files.

use super::*;
use beet_core::prelude::*;

/// Schedule label for route code generation systems.
///
/// This schedule runs all systems responsible for parsing route files,
/// creating route method entities, and generating codegen output.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub(crate) struct RouteCodegen;

impl RouteCodegen {
	/// Runs the route codegen schedule as a system.
	pub(crate) fn as_system() -> impl FnMut(&mut World) {
		|world: &mut World| {
			world.run_schedule(RouteCodegen);
		}
	}
}

/// Plugin that registers route code generation systems.
///
/// This plugin sets up the [`RouteCodegen`] schedule with all necessary systems
/// for processing route files and generating route handler code.
#[derive(Debug, Default, Clone)]
pub(crate) struct RouteCodegenPlugin;

impl Plugin for RouteCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_schedule(RouteCodegen).add_systems(
			RouteCodegen,
			(
				reset_codegen_files,
				create_route_files,
				// create the child routes
				parse_route_file_rs,
				parse_route_file_md,
				modify_route_file_tokens,
				tokenize_combinator_route,
				collect_route_files,
				// update codegen files
				reexport_child_codegen,
				parse_route_tree,
				// action codegen
				collect_client_action_group,
			)
				.chain(),
		);
	}
}
