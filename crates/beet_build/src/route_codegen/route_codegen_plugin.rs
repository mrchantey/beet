use crate::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;


/// Import files specified in each [`FileGroup`]
/// - Before [`ImportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportRouterCodegenStep;

/// Perform extra processing after files have been imported and processed.
/// - After [`ExportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessRouterCodegenStep;

/// Generate the [`CodegenFile`] for native files.
/// - After [`ProcessRouterCodegenStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportRouterCodegenStep;

#[derive(Debug, Default)]
pub struct RouteCodegenPlugin;


impl Plugin for RouteCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<RouteCodegenConfig>()
			.configure_sets(
				Update,
				(
					ImportRouterCodegenStep.before(ImportNodesStep),
					ProcessRouterCodegenStep.after(ExportNodesStep),
					ExportRouterCodegenStep.after(ProcessRouterCodegenStep),
				),
			)
			.add_systems(
				Update,
				(
					(
						spawn_route_files,
						(parse_route_file_rs, parse_route_file_md),
						modify_file_route_tokens,
					)
						.chain()
						.in_set(ImportRouterCodegenStep),
					(
						parse_route_tree,
						(
							(
								reexport_file_groups,
								add_client_codegen_to_actions_export,
							),
							collect_file_group,
						)
							.chain(),
						collect_client_action_group,
						(collect_combinator_route, tokenize_combinator_route)
							.chain(),
					)
						.in_set(ProcessRouterCodegenStep),
					#[cfg(not(test))]
					(
						// dont hit the fs in tests
						export_codegen_files,
						despawn_file_groups,
						compile_router,
					)
						.chain()
						.in_set(ExportRouterCodegenStep),
				),
			);
	}
}
