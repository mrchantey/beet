use super::*;
use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct RouteCodegenSequence;

impl Plugin for RouteCodegenSequence {
	fn build(&self, app: &mut App) {
		app
		.init_schedule(Self)
		.add_systems(
			Self,
			(
				reset_changed_codegen,
				update_route_files,
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


/// Marker type indicating the (usually `mod.rs`) file
/// containing reexports and static route trees.
/// This component is marked [`Changed`] when recompilation
/// is required.
#[derive(Debug, Clone, Default, Component)]
#[require(CodegenFile=default_codegen_file())]
pub struct RouteCodegenRoot;


fn default_codegen_file() -> CodegenFile {
	CodegenFile::new(
		AbsPathBuf::new_workspace_rel("src/codegen/mod.rs").unwrap(),
	)
}
