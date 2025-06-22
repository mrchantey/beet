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
pub struct RouterCodegenPlugin;


impl Plugin for RouterCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<RouterCodegenConfig>()
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
						reexport_file_groups,
						collect_file_group,
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
					)
						.chain()
						.in_set(ExportRouterCodegenStep),
				),
			);
	}
}
