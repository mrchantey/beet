use crate::prelude::*;
use anyhow::Result;
use beet_common::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub fn despawn_file_groups(
	mut commands: Commands,
	query: Populated<Entity, With<FileGroupSendit>>,
) {
	for entity in query.iter() {
		commands.entity(entity).despawn();
	}
}


/// Config included in the `beet.toml` file for a file group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Sendit)]
#[sendit(derive(Component))]
pub struct FileGroupConfig {
	/// Exclude the routes in this group from the route tree.
	/// Usually this should be true for pages and false for actions.
	#[serde(flatten)]
	pub file_group: FileGroup,
	#[serde(flatten)]
	pub codegen: Option<CodegenFile>,
	#[serde(flatten)]
	pub modifier: ModifyRouteFileMethod,
}


impl FileGroupConfig {
	pub fn into_bundle(self, parent_codegen: &CodegenFile) -> impl Bundle {
		let codegen = self.codegen.unwrap_or_else(|| {
			let default_out = self
				.file_group
				.src
				.parent()
				.unwrap_or(self.file_group.src.clone())
				.join("codegen.rs");
			parent_codegen.clone_meta(default_out)
		});
		(self.file_group.sendit(), codegen.sendit(), self.modifier)
	}
}

/// Definition for a group of files that should be collected together.
/// This is used as a field of types like [`ComponentFileGroup`] and [`RoutesFileGroup`].
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Sendit)]
#[sendit(derive(Component))]
#[sendit(require(CodegenFileSendit))]
pub struct FileGroup {
	/// Optionally set the group name, used for codegen file names
	/// like `FooRouterPlugin`, otherwise falls back to the
	/// [`CodegenFile::output`] filename.
	pub group_name: Option<String>,
	/// Passed to [`CodegenFile::pkg_name`]
	#[serde(rename = "package_name")]
	pub pkg_name: Option<String>,
	/// The directory where the files are located.
	#[serde(rename = "path")]
	pub src: AbsPathBuf,
	/// Include and exclude filters for the files.
	#[serde(flatten)]
	pub filter: GlobFilter,
	/// Specify the meta type, used for the file group codegen and individual
	/// route codegen like `.md` and `.rsx` files.
	#[serde(default = "unit_type", with = "syn_type_serde")]
	pub meta_type: syn::Type,
	#[serde(default = "unit_type", with = "syn_type_serde")]
	pub router_state_type: syn::Type,
	#[serde(default = "default_true")]
	pub route_tree: bool,
}

fn unit_type() -> syn::Type { syn::parse_str("()").unwrap() }
fn default_true() -> bool { true }

impl Default for FileGroup {
	fn default() -> Self {
		Self {
			group_name: None,
			pkg_name: None,
			route_tree: true,
			src: AbsPathBuf::default(),
			filter: GlobFilter::default(),
			meta_type: unit_type(),
			router_state_type: unit_type(),
		}
	}
}

impl FileGroup {
	pub fn new(src: AbsPathBuf) -> Self {
		Self {
			src,
			..Default::default()
		}
	}

	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	/// Perform a [`ReadDir`], returning all files in the directory
	/// relative this src
	pub fn collect_files(&self) -> Result<Vec<AbsPathBuf>> {
		let items = ReadDir {
			files: true,
			recursive: true,
			..Default::default()
		}
		.read(&self.src)?
		.into_iter()
		.filter_map(|path| {
			if self.filter.passes(&path) {
				// should be path+self.src?
				Some(AbsPathBuf::new(path))
			} else {
				None
			}
		})
		.collect::<Result<Vec<_>, FsError>>()?;
		Ok(items)
	}

	#[cfg(test)]
	pub fn test_site() -> impl Bundle {
		(
			Self::new(
				WsPathBuf::new("crates/beet_router/src/test_site").into_abs(),
			)
			.sendit(),
			CodegenFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/codegen/mod.rs",
				)
				.into_abs(),
			)
			.with_pkg_name("test_site")
			.sendit(),
		)
	}
	#[cfg(test)]
	pub fn test_site_pages() -> impl Bundle {
		(
			Self::new(
				WsPathBuf::new("crates/beet_router/src/test_site/pages")
					.into_abs(),
			)
			.with_filter(
				GlobFilter::default()
					.with_include("*.rs")
					.with_exclude("*mod.rs"),
			)
			.sendit(),
			CodegenFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/codegen/pages.rs",
				)
				.into_abs(),
			)
			.with_pkg_name("test_site")
			.sendit(),
		)
	}
	#[cfg(test)]
	pub fn test_site_docs() -> impl Bundle {
		(
			Self::new(
				WsPathBuf::new("crates/beet_router/src/test_site/test_docs")
					.into_abs(),
			)
			.with_filter(GlobFilter::default().with_include("*.md"))
			.sendit(),
			CodegenFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/codegen/test_docs.rs",
				)
				.into_abs(),
			)
			.with_pkg_name("test_site")
			.sendit(),
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::prelude::GlobFilter;
	use beet_utils::prelude::WsPathBuf;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(
			FileGroup::new(
				WsPathBuf::new("crates/beet_router/src/test_site").into_abs(),
			)
			.with_filter(GlobFilter::default().with_include("*.mockup.rs"))
			.collect_files()
			.unwrap()
			.len(),
		)
		.to_be(2);
	}
}
