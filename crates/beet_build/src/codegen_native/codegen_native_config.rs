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
pub struct CodegenNativeConfig {
	/// The name of the package being built, used for imports in codegen.
	/// This will be applied to each [`FileGroup::pkg_name`] if it is None.
	#[serde(flatten)]
	pub codegen_file: CodegenFile,
	/// Optionally set the path for the docs route.
	/// By default this is set to `/docs` but if your entire site is a docs
	/// site it may be more idiomatic to set this to `None`.
	#[serde(default = "default_docs_route")]
	pub docs_route: String,
	/// Disable the default file groups: `pages`, `docs`, and `actions`.
	/// Also disables the [`ParseRouteTree`] modifier.
	#[serde(default)]
	pub no_defaults: bool,
	/// Additional file groups to be included in the codegen.
	#[serde(default, rename = "file_group")]
	pub file_groups: Vec<FileGroupConfig>,
}

fn default_docs_route() -> String { "/".to_string() }

fn default_codegen_file() -> CodegenFile {
	CodegenFile::new(
		AbsPathBuf::new_workspace_rel("src/codegen/mod.rs").unwrap(),
	)
	.with_pkg_name(
		std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "beet".to_string()),
	)
}

impl Default for CodegenNativeConfig {
	fn default() -> Self {
		Self {
			codegen_file: default_codegen_file(),
			docs_route: default_docs_route(),
			no_defaults: false,
			file_groups: Vec::new(),
		}
	}
}

impl NonSendPlugin for CodegenNativeConfig {
	fn build(mut self, app: &mut App) {
		self.try_append_default_groups();

		let mut root =
			app.world_mut().spawn(self.codegen_file.clone().sendit());
		if !self.no_defaults {
			root.insert(ParseRouteTree);
		}
		for group in self.file_groups {
			root.with_child(group.into_bundle(&self.codegen_file));
		}
	}
}


impl CodegenNativeConfig {
	fn try_append_default_groups(&mut self) {
		if self.no_defaults {
			return;
		}
		if let Some(pages) = self.default_group("pages") {
			self.file_groups.push(pages);
		}
		if let Some(mut docs) = self.default_group("docs") {
			docs.modifier.base_route = Some(self.docs_route.clone().into());
			self.file_groups.push(docs);
		}
		if let Some(mut actions) = self.default_group("actions") {
			actions.file_group.route_tree = false;
			// TODO insert additional parse_actions modifier
			self.file_groups.push(actions);
		}
	}


	fn default_group(&self, name: &str) -> Option<FileGroupConfig> {
		// from src/codegen/mod.rs to src/<name>.rs
		let group_dir = self.codegen_file.output.parent()?.parent()?.join(name);

		if !group_dir.exists() {
			return None;
		}
		let codegen_path = self
			.codegen_file
			.output
			.parent()?
			.join(format!("{name}.rs"));

		Some(FileGroupConfig {
			file_group: FileGroup::new(group_dir),
			codegen: Some(self.codegen_file.clone_meta(codegen_path)),
			modifier: Default::default(),
		})
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(CodegenNativePlugin).update();
	}
}
