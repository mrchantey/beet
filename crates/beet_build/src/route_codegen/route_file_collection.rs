use crate::prelude::*;
use beet_core::as_beet::*;
use beet_net::prelude::*;
use beet_utils::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Added alongside a [`SourceFile`] for easy cohersion of route meta
#[derive(Debug, PartialEq, Clone, Component)]
pub struct MetaType(pub Unspan<syn::Type>);

impl MetaType {
	pub fn new(ty: syn::Type) -> Self { Self(Unspan::new(&ty)) }
}

/// Definition for a group of route files that should be collected together,
/// including pages and actions.
#[derive(Debug, PartialEq, Clone, Component)]
#[require(CodegenFile)]
pub struct RouteFileCollection {
	/// The directory where the files are located.
	pub src: AbsPathBuf,
	/// Include and exclude filters for the files.
	pub filter: GlobFilter,
	pub category: RouteCollectionCategory,
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

	/// Get the
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

/// Create a [`SourceFile`] for each file in a [`RouteFileCollection`].
pub fn import_route_file_collection(
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


/// Whenever a [`SourceFile`] is created, reparent to a corresponding [`RouteFileCollection`],
/// if any.
pub fn reparent_route_collection_source_files(
	mut commands: Commands,
	query: Populated<(Entity, &SourceFile), Added<SourceFile>>,
	collections: Query<(Entity, &RouteFileCollection)>,
) -> Result {
	// a hashmap mapping every file in a collection to that collection entity
	let mut file_collection_map: HashMap<AbsPathBuf, Entity> =
		HashMap::default();
	for (entity, collection) in collections.iter() {
		for file in collection.read_files()? {
			if file_collection_map.contains_key(&file) {
				bevybail!(
					"
Error: Collection Overlap: {}
This file appears in multple collections,
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
	pub fn cache_strategy(&self) -> CacheStrategy {
		match self {
			Self::Pages => CacheStrategy::Static,
			Self::Actions => CacheStrategy::Dynamic,
		}
	}
}

impl RouteFileCollection {
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
			children![SourceFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/pages/docs/index.rs",
				)
				.into_abs(),
			)],
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
			children![SourceFile::new(
				WsPathBuf::new(
					"crates/beet_router/src/test_site/test_docs/index.mdx",
				)
				.into_abs(),
			)],
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
			.xpect_true();
		collection
			.passes_filter(&WsPathBuf::new("foobar/index.mockup.rs").into_abs())
			.xpect_false();
		collection
			.passes_filter(
				&WsPathBuf::new("crates/beet_router/src/test_site/index.rs")
					.into_abs(),
			)
			.xpect_false();
	}
}
