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
	#[serde(rename = "package_name")]
	pub pkg_name: String,
	#[serde(default = "default_src_path")]
	pub src_path: AbsPathBuf,
	/// Optionally set the path for the docs route.
	/// By default this is set to `/docs` but if your entire site is a docs
	/// site it may be more idiomatic to set this to `None`.
	#[serde(default = "default_docs_route")]
	pub docs_route: String,
	/// Disable the default file groups: `pages`, `docs`, and `actions`.
	#[serde(default)]
	pub no_default_groups: bool,
	/// Additional file groups to be included in the codegen.
	#[serde(default, rename = "file_group")]
	pub file_groups: Vec<FileGroupConfig>,
}
fn default_src_path() -> AbsPathBuf {
	AbsPathBuf::new_workspace_rel("src").unwrap()
}
fn default_docs_route() -> String { "/".to_string() }

impl Default for CodegenNativeConfig {
	fn default() -> Self {
		Self {
			no_default_groups: false,
			pkg_name: std::env::var("CARGO_PKG_NAME")
				.unwrap_or_else(|_| "beet".to_string()),
			src_path: default_src_path(),
			file_groups: Vec::new(),
			docs_route: default_docs_route(),
		}
	}
}

impl NonSendPlugin for CodegenNativeConfig {
	fn build(mut self, app: &mut App) {
		self.try_append_default_groups();

		let mut root = app.world_mut().spawn_empty();
		for mut group in self.file_groups {
			if group.codegen.pkg_name.is_none() {
				group.codegen.pkg_name = Some(self.pkg_name.clone());
			}
			root.with_child(group.into_bundle());
		}
	}
}


impl CodegenNativeConfig {
	fn try_append_default_groups(&mut self) {
		if self.no_default_groups {
			return;
		}
		if let Some(pages) = self.default_group("pages") {
			self.file_groups.push(pages);
		}
		if let Some(mut docs) = self.default_group("docs") {
			docs.modifier.base_route = Some(self.docs_route.clone().into());
			self.file_groups.push(docs);
		}
		if let Some(actions) = self.default_group("actions") {
			// TODO insert additional parse_actions modifier
			self.file_groups.push(actions);
		}
	}


	fn default_group(&self, name: &str) -> Option<FileGroupConfig> {
		let path = self.src_path.join(name);
		if !path.exists() {
			return None;
		}

		Some(FileGroupConfig {
			file_group: FileGroup::new(path),
			codegen: CodegenFile::new(
				self.src_path.join(format!("codegen/{name}.rs")),
			),
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
