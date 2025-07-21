use crate::prelude::*;
use beet_core::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Definition for a group of route files that should be collected together,
/// including pages and actions.
#[derive(Debug, PartialEq, Clone, Component)]
#[require(CodegenFile)]
pub struct RouteFileCollection {
	/// The directory where the files are located.
	pub src: AbsPathBuf,
	/// Include and exclude filters for the files.
	pub filter: GlobFilter,
	/// Specify the meta type, used for the file group codegen and individual
	/// route codegen like `.md` and `.rsx` files.
	pub meta_type: Unspan<syn::Type>,
	pub router_state_type: Unspan<syn::Type>,
	pub category: RouteCollectionCategory,
}

impl Default for RouteFileCollection {
	fn default() -> Self {
		Self {
			src: Default::default(),
			filter: Default::default(),
			meta_type: Unspan::parse_str("()").unwrap(),
			router_state_type: Unspan::parse_str("()").unwrap(),
			category: Default::default(),
		}
	}
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum RouteCollectionCategory {
	/// Files contain public functions named after the http methods,
	/// and will be included in the route tree.
	#[default]
	Pages,
	/// Files contain arbitary axum routes,
	/// and will be excluded from the route tree.
	Actions,
}

impl RouteCollectionCategory {
	pub fn include_in_route_tree(&self) -> bool {
		match self {
			Self::Pages => true,
			Self::Actions => false,
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
