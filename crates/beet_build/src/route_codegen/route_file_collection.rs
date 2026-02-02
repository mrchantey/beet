//! Route file collection configuration for grouping related route files.
//!
//! This module defines the [`RouteFileCollection`] component that groups
//! source files together for route codegen, such as all pages or all actions.

use crate::prelude::*;
use beet_core::prelude::*;
use quote::ToTokens;

/// Added alongside a [`SourceFile`] for easy coercion of route metadata.
#[derive(Debug, PartialEq, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct MetaType(String);

impl MetaType {
	/// Creates a new meta type from a syn type.
	pub fn new(ty: syn::Type) -> Self {
		Self(ty.into_token_stream().to_string())
	}

	/// Returns the inner syn type.
	pub fn inner(&self) -> syn::Type {
		syn::parse_str(&self.0).expect("MetaType contained invalid syn::Type")
	}
}

/// Defines a group of route files that should be collected together.
///
/// This includes configuration for pages and actions, specifying the source
/// directory and filters for which files to include.
#[derive(Debug, PartialEq, Clone, Reflect, Component)]
#[reflect(Component)]
#[require(CodegenFile)]
pub struct RouteFileCollection {
	/// The directory where the source files are located.
	pub src: AbsPathBuf,
	/// Include and exclude filters for the files.
	pub filter: GlobFilter,
	/// The category of routes in this collection.
	pub category: RouteCollectionCategory,
}

impl RouteFileCollection {
	/// Creates a new route file collection from a source directory.
	pub fn new(src: AbsPathBuf) -> Self {
		Self {
			src,
			..Default::default()
		}
	}

	/// Sets the glob filter for this collection.
	pub fn with_filter(mut self, filter: GlobFilter) -> Self {
		self.filter = filter;
		self
	}

	/// Returns `true` if the given path passes the collection's filter.
	pub fn passes_filter(&self, path: &AbsPathBuf) -> bool {
		path.starts_with(&self.src) && self.filter.passes(path)
	}

	/// Reads all files matching this collection's filter.
	fn read_files(&self) -> Result<Vec<AbsPathBuf>> {
		let mut files = Vec::new();
		for file in ReadDir::files_recursive(&self.src)? {
			let abs_path = AbsPathBuf::new(file)?;
			if self.passes_filter(&abs_path) {
				files.push(abs_path);
			}
		}
		Ok(files)
	}
}

/// Creates a [`SourceFile`] for each file in a [`RouteFileCollection`].
#[allow(unused)]
pub(crate) fn import_route_file_collection(
	mut commands: Commands,
	collections: Query<
		(Entity, &RouteFileCollection),
		Added<RouteFileCollection>,
	>,
) -> Result {
	for (entity, collection) in collections.iter() {
		for file in ReadDir::files_recursive(&collection.src)? {
			let file = AbsPathBuf::new(file)?;
			commands.spawn((ChildOf(entity), SourceFile::new(file)));
		}
	}
	Ok(())
}


/// Reparents [`SourceFile`] entities to their corresponding [`RouteFileCollection`].
///
/// Whenever a [`SourceFile`] is created, this system checks if it belongs to
/// any route file collection and reparents it accordingly.
pub(crate) fn reparent_route_collection_source_files(
	mut commands: Commands,
	query: Populated<(Entity, &SourceFile), Added<SourceFile>>,
	collections: Query<(Entity, &RouteFileCollection)>,
) -> Result {
	// a hashmap mapping every file in a collection to that collection entity
	let mut file_collection_map: HashMap<AbsPathBuf, Entity> =
		HashMap::default();
	for (entity, collection) in collections.iter() {
		for file in collection.read_files()? {
			if let Some(existing) = file_collection_map.get(&file) {
				assert_ne!(entity, *existing);
				let collection2 = collections.get(*existing).unwrap().1;
				bevybail!(
					"
Error: Collection Overlap: {}
This file appears in multiple collections:
Collection A: {collection2:#?}
Collection B: {collection:#?}
Please constrain the collection filters or roots",
					file
				);
			}
			file_collection_map.insert(file, entity);
		}
	}
	// for each source file, insert as a child of the collection entity if it exists
	for (entity, source_file) in query.iter() {
		if let Some(collection_entity) = file_collection_map.get(&**source_file)
		{
			commands.entity(entity).insert(ChildOf(*collection_entity));
		}
	}
	Ok(())
}

impl Default for RouteFileCollection {
	fn default() -> Self {
		Self {
			src: Default::default(),
			filter: Default::default(),
			category: Default::default(),
		}
	}
}

/// Categorizes route files by their purpose.
#[derive(Debug, Default, Copy, Clone, PartialEq, Reflect)]
pub enum RouteCollectionCategory {
	/// Files contain public functions named after HTTP methods,
	/// and will be included in the route tree.
	#[default]
	Pages,
	/// Files contain arbitrary routes,
	/// and will be excluded from the route tree.
	Actions,
}

impl RouteCollectionCategory {
	/// Returns `true` if routes in this category should be included in the route tree.
	pub fn include_in_route_tree(&self) -> bool {
		match self {
			Self::Pages => true,
			Self::Actions => false,
		}
	}

	/// Returns the cache strategy for routes in this category.
	pub fn cache_strategy(&self) -> CacheStrategy {
		match self {
			Self::Pages => CacheStrategy::Static,
			Self::Actions => CacheStrategy::Dynamic,
		}
	}
}

impl RouteFileCollection {
	/// Creates a test site route file collection for testing.
	#[cfg(test)]
	pub fn test_site() -> impl Bundle {
		(
			Self::new(WsPathBuf::new("tests/test_site").into_abs()),
			CodegenFile::new(
				WsPathBuf::new("tests/test_site/codegen/mod.rs").into_abs(),
			)
			.with_pkg_name("test_site"),
		)
	}

	/// Creates a test site pages collection for testing.
	#[cfg(test)]
	pub fn test_site_pages() -> impl Bundle {
		(
			Self::new(WsPathBuf::new("tests/test_site/pages").into_abs())
				.with_filter(
					GlobFilter::default()
						.with_include("*.rs")
						.with_exclude("*mod.rs"),
				),
			CodegenFile::new(
				WsPathBuf::new("tests/test_site/codegen/pages.rs").into_abs(),
			)
			.with_pkg_name("test_site"),
			children![SourceFile::new(
				WsPathBuf::new("tests/test_site/pages/docs/index.rs",)
					.into_abs(),
			)],
		)
	}

	/// Creates a test site docs collection for testing.
	#[cfg(test)]
	pub fn test_site_docs() -> impl Bundle {
		(
			Self::new(WsPathBuf::new("tests/test_site/test_docs").into_abs())
				.with_filter(
					GlobFilter::default()
						.with_include("*.md")
						.with_include("*.mdx"),
				),
			CodegenFile::new(
				WsPathBuf::new("tests/test_site/codegen/test_docs.rs")
					.into_abs(),
			)
			.with_pkg_name("test_site"),
			children![SourceFile::new(
				WsPathBuf::new("tests/test_site/test_docs/index.mdx",)
					.into_abs(),
			)],
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		let collection = RouteFileCollection::new(
			WsPathBuf::new("tests/test_site").into_abs(),
		)
		.with_filter(GlobFilter::default().with_include("*.mockup.rs"));

		collection
			.passes_filter(
				&WsPathBuf::new("tests/test_site/index.mockup.rs").into_abs(),
			)
			.xpect_true();
		collection
			.passes_filter(&WsPathBuf::new("foobar/index.mockup.rs").into_abs())
			.xpect_false();
		collection
			.passes_filter(
				&WsPathBuf::new("tests/test_site/index.rs").into_abs(),
			)
			.xpect_false();
	}
}
