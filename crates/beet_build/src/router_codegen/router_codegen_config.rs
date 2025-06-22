use crate::prelude::*;
use beet_bevy::prelude::NonSendPlugin;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// The default codegen builder for a beet site.
///
/// This will perform the following tasks:
///
/// - If a `src/actions` dir exists, generate server actions
/// - If a `src/pages` dir exists, generate pages codegen and add to the route tree
/// - If a `src/docs` dir exists, generate docs codegen and add to the route tree
///
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterCodegenConfig {
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

impl Default for RouterCodegenConfig {
	fn default() -> Self {
		Self {
			codegen_file: default_codegen_file(),
			file_groups: Vec::new(),
		}
	}
}

impl NonSendPlugin for RouterCodegenConfig {
	fn build(self, app: &mut App) {
		let mut root = app.world_mut().spawn((
			RouterCodegenRoot::default(),
			self.codegen_file.clone().sendit(),
		));
		root.with_children(|mut parent| {
			for group in self.file_groups {
				group.spawn(&mut parent);
			}
		});
	}
}

#[derive(Debug, Clone, Default, Component)]
#[require(CodegenFileSendit)]
pub struct RouterCodegenRoot {}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(RouterCodegenPlugin).update();
	}
}
