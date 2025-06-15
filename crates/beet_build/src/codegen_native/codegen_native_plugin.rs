use crate::prelude::*;
use beet_parse::prelude::*;
use bevy::prelude::*;


/// Import files specified in each [`FileGroup`]
/// - Before [`ImportNodesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ImportCodegenNativeStep;

/// Perform extra processing after files have been imported and processed.
/// - After [`ExportNodesStep`]
/// - After [`ImportCodegenNativesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ProcessCodegenNativeStep;

/// Generate the [`CodegenFile`] for native files.
/// - After [`ProcessCodegenNativesStep`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExportCodegenNativeStep;


pub struct CodegenNativePlugin;


impl Plugin for CodegenNativePlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<CodegenNativeConfig>()
			.configure_sets(
				Update,
				(
					ImportTemplateStep.before(ImportNodesStep),
					ProcessTemplateStep
						.after(ExportNodesStep)
						.after(ImportTemplateStep),
					ExportTemplateStep.after(ProcessTemplateStep),
				),
			)
			.add_systems(
				Update,
				(
					spawn_route_files,
					parse_route_file_rust,
					// (parse_route_file_rust, parse_route_file_markdown),
					modify_file_route_tokens,
				)
					.chain()
					.in_set(ImportCodegenNativeStep),
			);
	}
}
