use crate::prelude::*;
use beet_bevy::prelude::NonSendPlugin;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

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
		let mut root = app.world_mut().spawn((
			RouteCodegenRoot::default(),
			self.codegen_file.clone().sendit(),
		));
		root.with_children(|mut parent| {
			for group in self.file_groups {
				group.spawn(&mut parent);
			}
		});
	}
}
/// Marker type indicating the (usually `mod.rs`) file
/// containing reexports and static route trees.
#[derive(Debug, Clone, Default, Component)]
#[require(CodegenFileSendit)]
pub struct RouteCodegenRoot;
