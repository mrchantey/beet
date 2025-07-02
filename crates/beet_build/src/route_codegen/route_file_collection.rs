use crate::prelude::*;
use beet_common::as_beet::*;
use beet_utils::prelude::*;
use bevy::ecs::relationship::RelatedSpawner;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Config included in the `beet.toml` file for a collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteFileConfig {
	/// Exclude the routes in this collection from the route tree.
	/// Usually this should be true for pages and false for actions.
	#[serde(flatten)]
	pub collection: RouteFileCollection,
	#[serde(flatten)]
	pub codegen: CodegenFile,
	#[serde(flatten)]
	pub modify_route: ModifyRoutePath,
}


impl RouteFileConfig {
	pub fn spawn(self, spawner: &mut RelatedSpawner<ChildOf>) -> impl Bundle {
		let client_actions_codegen =
			if self.collection.category == RouteFileCategory::Action {
				let codegen = self.codegen.clone_info(
					CollectClientActions::path(&self.codegen.output),
				);
				Some(codegen)
			} else {
				None
			};

		let mut entity =
			spawner.spawn((self.collection, self.codegen, self.modify_route));
		if let Some(client_actions_codegen) = client_actions_codegen {
			entity.with_child((
				client_actions_codegen,
				CollectClientActions::default(),
			));
		}
	}
}

/// Definition for a group of route files that should be collected together,
/// including pages and actions.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Component)]
#[require(CodegenFile)]
pub struct RouteFileCollection {
	/// Optionally set the group name, used for codegen file names
	/// like `FooRouterPlugin`, otherwise falls back to the
	/// [`CodegenFile::output`] filename.
	#[serde(rename = "name")]
	pub name: Option<String>,
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
	pub meta_type: Unspan<syn::Type>,
	#[serde(default = "unit_type", with = "syn_type_serde")]
	pub router_state_type: Unspan<syn::Type>,
	#[serde(default)]
	pub category: RouteFileCategory,
}


#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum RouteFileCategory {
	/// Files contain public functions named after the http methods,
	/// and will be included in the route tree.
	#[default]
	Page,
	/// Files contain arbitary axum routes,
	/// and will be excluded from the route tree.
	Action,
}

impl RouteFileCategory {
	pub fn include_in_route_tree(&self) -> bool {
		match self {
			Self::Page => true,
			Self::Action => false,
		}
	}
}

fn unit_type() -> Unspan<syn::Type> { Unspan::parse_str("()").unwrap() }

impl Default for RouteFileCollection {
	fn default() -> Self {
		Self {
			name: None,
			pkg_name: None,
			category: Default::default(),
			src: Default::default(),
			filter: Default::default(),
			meta_type: unit_type(),
			router_state_type: unit_type(),
		}
	}
}

impl RouteFileCollection {
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

	pub fn passes_filter(&self, path: &AbsPathBuf) -> bool {
		path.starts_with(&self.src) && self.filter.passes(path)
	}

	#[cfg(test)]
	pub fn test_site() -> impl Bundle {
		(
			Self::new(
				WsPathBuf::new("crates/beet_router/src/test_site").into_abs(),
			),
			CodegenFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/codegen/mod.rs",
				)
				.into_abs(),
			)
			.with_pkg_name("test_site"),
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
			),
			CodegenFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/codegen/pages.rs",
				)
				.into_abs(),
			)
			.with_pkg_name("test_site"),
		)
	}
	#[cfg(test)]
	pub fn test_site_docs() -> impl Bundle {
		(
			Self::new(
				WsPathBuf::new("crates/beet_router/src/test_site/test_docs")
					.into_abs(),
			)
			.with_filter(
				GlobFilter::default()
					.with_include("*.md")
					.with_include("*.mdx"),
			),
			CodegenFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/codegen/test_docs.rs",
				)
				.into_abs(),
			)
			.with_pkg_name("test_site"),
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
		let collection = RouteFileCollection::new(
			WsPathBuf::new("crates/beet_router/src/test_site").into_abs(),
		)
		.with_filter(GlobFilter::default().with_include("*.mockup.rs"));

		collection
			.passes_filter(
				&WsPathBuf::new(
					"crates/beet_router/src/test_site/index.mockup.rs",
				)
				.into_abs(),
			)
			.xpect()
			.to_be_true();
		collection
			.passes_filter(&WsPathBuf::new("foobar/index.mockup.rs").into_abs())
			.xpect()
			.to_be_false();
		collection
			.passes_filter(
				&WsPathBuf::new("crates/beet_router/src/test_site/index.rs")
					.into_abs(),
			)
			.xpect()
			.to_be_false();
	}
}
