use crate::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;


/// Import files specified in each [`FileGroup`]
/// - Before [`ImportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportCodegenNativeStep;

/// Perform extra processing after files have been imported and processed.
/// - After [`ExportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessCodegenNativeStep;

/// Generate the [`CodegenFile`] for native files.
/// - After [`ProcessCodegenNativeStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportCodegenNativeStep;

#[derive(Debug, Default)]
pub struct CodegenNativePlugin;


impl Plugin for CodegenNativePlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<CodegenNativeConfig>()
			.configure_sets(
				Update,
				(
					ImportCodegenNativeStep.before(ImportNodesStep),
					ProcessCodegenNativeStep.after(ExportNodesStep),
					ExportCodegenNativeStep.after(ProcessCodegenNativeStep),
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
						.in_set(ImportCodegenNativeStep),
					(
						parse_route_tree,
						collect_file_group,
						collect_client_action_group,
						(collect_combinator_route, tokenize_combinator_route)
							.chain(),
					)
						.in_set(ProcessCodegenNativeStep),
					#[cfg(not(test))]
					(
						// dont hit the fs in tests
						export_codegen_files,
						despawn_file_groups,
					)
						.chain()
						.in_set(ExportCodegenNativeStep),
				),
			);
	}
}
