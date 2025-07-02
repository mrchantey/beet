use crate::prelude::*;
use beet_bevy::prelude::NonSendPlugin;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Default)]
pub struct RouteCodegenPlugin;

impl Plugin for RouteCodegenPlugin {
	fn build(&self, app: &mut App) {
		app.init_non_send_resource::<RouteCodegenConfig>()
			.add_systems(
				Update,
				(
					// (
					spawn_route_files,
					// 	(parse_route_file_rs, parse_route_file_md),
					// 	modify_file_route_tokens,
					// )
					// 	.chain()
					// 	.in_set(BeforeParseTokens),
					// (
					// 	parse_route_tree,
					// 	(
					// 		(
					// 			reexport_file_groups,
					// 			add_client_codegen_to_actions_export,
					// 		),
					// 		collect_file_group,
					// 	)
					// 		.chain(),
					// 	collect_client_action_group,
					// 	(collect_combinator_route, tokenize_combinator_route)
					// 		.chain(),
					// )
					// 	.in_set(AfterParseTokens),
					// #[cfg(not(test))]
					// compile_router.after(ExportArtifactsSet),
				)
					.run_if(|flags: Res<BuildFlags>| {
						flags.contains(BuildFlag::Routes)
					}),
			);
	}
}

/// The codegen builder for routes in a beet site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteCodegenConfig {
	/// The root codegen, containing the route mod tree and other utilities.
	#[serde(flatten)]
	pub codegen_file: CodegenFile,
	/// Additional file groups to be included in the codegen.
	#[serde(default, rename = "file_group")]
	pub file_groups: Vec<FileGroupConfig>,
}


fn default_codegen_file() -> CodegenFile {
	CodegenFile::new(
		AbsPathBuf::new_workspace_rel("src/codegen/mod.rs").unwrap(),
	)
	.with_pkg_name(
		std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "beet".to_string()),
	)
}

impl Default for RouteCodegenConfig {
	fn default() -> Self {
		Self {
			codegen_file: default_codegen_file(),
			file_groups: Vec::new(),
		}
	}
}

impl NonSendPlugin for RouteCodegenConfig {
	fn build(self, app: &mut App) {
		let mut root = app
			.world_mut()
			.spawn((RouteCodegenRoot::default(), self.codegen_file.clone()));
		root.with_children(|mut parent| {
			for group in self.file_groups {
				group.spawn(&mut parent);
			}
		});
	}
}
/// Marker type indicating the (usually `mod.rs`) file
/// containing reexports and static route trees.
/// This component will be marked Changed when recompilation
/// is required.
#[derive(Debug, Clone, Default, Component)]
#[require(CodegenFile)]
pub struct RouteCodegenRoot;
