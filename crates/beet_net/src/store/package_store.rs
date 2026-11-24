//! [`PackageStore`]: where a document's references resolve.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// The store rooted at the directory holding a package's entry, which every
/// [`DocPath`] at or under this entity resolves against.
///
/// Landed on an entry's root by its load, and on the root of a tree built
/// apart from it (a page rendering for one request) by whatever builds it.
#[derive(Debug, Clone, Deref, Component)]
pub struct PackageStore(BlobStore);

impl PackageStore {
	/// The package whose entry is at `entry` in `repo_store`.
	pub fn of_entry(repo_store: &BlobStore, entry: &RelPath) -> Self {
		match entry.parent().filter(|dir| !dir.is_empty()) {
			Some(dir) => Self(repo_store.with_subdir(dir)),
			None => Self(repo_store.clone()),
		}
	}

	/// The package `store` is rooted at.
	pub fn new(store: BlobStore) -> Self { Self(store) }

	/// The path of `blob` in this package, its path in its own store when that
	/// store is elsewhere.
	pub fn path_of(&self, blob: &Blob) -> RelPath {
		let within = blob.store().root_key() == self.root_key();
		let full = blob.store().subdir().join(blob.path());
		within
			.then(|| match self.subdir().is_empty() {
				true => Some(full.clone()),
				false => full.strip_prefix(&self.subdir()),
			})
			.flatten()
			.unwrap_or_else(|| blob.path().clone())
	}
}

/// The package an entity's references resolve against: the nearest
/// [`PackageStore`] at or above it, else, for a tree no entry loaded, the
/// outermost [`BlobStore`], since one nearer is scoped within it (a
/// [`DirPath`]).
#[derive(SystemParam)]
pub struct PackageQuery<'w, 's> {
	packages: AncestorQuery<'w, 's, &'static PackageStore>,
	parents: Query<'w, 's, &'static ChildOf>,
	stores: Query<'w, 's, &'static BlobStore>,
}

impl PackageQuery<'_, '_> {
	/// The package of `entity`, an error when nothing above it holds a store.
	pub fn get(&self, entity: Entity) -> Result<PackageStore> {
		match self.packages.get(entity) {
			Ok(package) => package.clone().xok(),
			Err(_) => core::iter::once(entity)
				.chain(self.parents.iter_ancestors(entity))
				.filter_map(|entity| self.stores.get(entity).ok())
				.last()
				.map(|store| PackageStore::new(store.clone()))
				.ok_or_else(|| {
					bevyhow!(
						"no store above {entity} to read its documents from: \
						an entry load composes one on its root"
					)
				}),
		}
	}
}
