use super::*;
use crate::prelude::*;
use beet_core::prelude::NonSendPlugin;
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
				// not a perfect ordering, some of these, ie action codegen, arent actually dependent on preceeding
				// systems but legibility is more valuable than perf at this stage
				((
					(reset_changed_codegen, update_route_files),
					// create the child routes
					(parse_route_file_rs, parse_route_file_md),
					modify_route_file_tokens,
					(collect_combinator_route_meta, tokenize_combinator_route),
					collect_route_files,
					// update root codegen file
					reexport_collections,
					parse_route_tree,
					// action codegen
					(
						add_client_codegen_to_actions_export,
						collect_client_action_group,
					),
				)
					.chain()
					.in_set(ProcessChangedSnippets))
				.run_if(BuildFlags::should_run(BuildFlag::Routes)),
			);
	}
}


/// Call [`CodegenFile::build_and_write`] for every [`Changed<CodegenFile>`]
pub fn export_route_codegen(
	query: Populated<&CodegenFile, Changed<CodegenFile>>,
) -> bevy::prelude::Result {
	let num_files = query.iter().count();
	info!("Exporting {} codegen files...", num_files);
	for codegen_file in query.iter() {
		codegen_file.build_and_write()?;
	}
	Ok(())
}


/// The codegen builder for routes in a beet site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteCodegenConfig {
	/// The root codegen, containing the route mod tree and other utilities.
	#[serde(flatten)]
	pub codegen_file: CodegenFile,
	/// Collections to be included in the codegen.
	#[serde(default, rename = "collection")]
	pub collections: Vec<RouteFileConfig>,
}


fn default_codegen_file() -> CodegenFile {
	CodegenFile::new(
		AbsPathBuf::new_workspace_rel("src/codegen/mod.rs").unwrap(),
	)
}

impl Default for RouteCodegenConfig {
	fn default() -> Self {
		Self {
			codegen_file: default_codegen_file(),
			collections: Vec::new(),
		}
	}
}

impl NonSendPlugin for RouteCodegenConfig {
	fn build(self, app: &mut App) {
		let mut root = app
			.world_mut()
			.spawn((RouteCodegenRoot::default(), self.codegen_file.clone()));
		root.with_children(|mut parent| {
			for collection in self.collections {
				collection.spawn(&mut parent);
			}
		});
	}
}
/// Marker type indicating the (usually `mod.rs`) file
/// containing reexports and static route trees.
/// This component is marked [`Changed`] when recompilation
/// is required.
#[derive(Debug, Clone, Default, Component)]
#[require(CodegenFile)]
pub struct RouteCodegenRoot;
