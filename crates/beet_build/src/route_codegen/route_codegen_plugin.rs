use super::*;
use beet_core::prelude::*;

/// Schedule label for route code generation systems.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RouteCodegen;

/// Plugin that registers route code generation systems.
#[derive(Debug, Default, Clone)]
pub struct RouteCodegenPlugin;


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
