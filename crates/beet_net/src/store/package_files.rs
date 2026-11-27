//! [`PackageFiles`]: what a [`DocPath`] resolves in.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;

/// A package's files: the store rooted at the directory holding its entry,
/// which every [`DocPath`] at or under this entity resolves in
/// ([`PackageFilesQuery::resolve`]). The runtime's own package is the
/// [`PackageStore`] scoped to its entry's directory; a package loaded beside
/// it carries its own.
///
/// A field's type says which store its path is of: a [`DocPath`] names a file
/// of the package, whatever scoped store it is written under, and any other
/// path one in the nearest [`BlobStore`], ie what a toolset reads and writes
/// beneath a [`DirPath`]. Nothing hands out the package's files as a store, so no
/// reader picks between them.
///
/// Landed on an entry's root by its load, and on the root of a tree built
/// apart from it (a page rendering for one request) by whatever builds it.
#[derive(Debug, Clone, Component)]
pub struct PackageFiles(BlobStore);

impl PackageFiles {
	/// The package whose entry is at `entry` in `package_store`.
	pub fn of_entry(package_store: &BlobStore, entry: &FwdPath) -> Self {
		match entry.parent().filter(|dir| !dir.is_empty()) {
			Some(dir) => Self(package_store.with_subdir(dir)),
			None => Self(package_store.clone()),
		}
	}

	/// The package `store` is rooted at.
	pub fn new(store: BlobStore) -> Self { Self(store) }

	/// The file `document` names, its extension as written
	/// ([`Blob::complete`]).
	///
	/// # Errors
	/// Errors when the reference is no path within the package.
	pub fn resolve(&self, document: &DocPath) -> Result<Blob> {
		self.0.blob(document.fwd_path()?).xok()
	}

	/// The path of `blob` in the package, its path in its own store when that
	/// store is elsewhere: the inverse of [`resolve`](Self::resolve).
	pub fn path_of(&self, blob: &Blob) -> FwdPath {
		let subdir = self.0.subdir();
		let full = blob.store().subdir().join(blob.path());
		(blob.store().root_key() == self.0.root_key())
			.then(|| match subdir.is_empty() {
				true => Some(full.clone()),
				false => full.strip_prefix(&subdir),
			})
			.flatten()
			.unwrap_or_else(|| blob.path().clone())
	}
}

/// The package an entity's references resolve against: the nearest
/// [`PackageFiles`] at or above it, else, for a tree no entry loaded, the
/// outermost [`BlobStore`], since one nearer is scoped within it (a
/// [`DirPath`]).
#[derive(SystemParam)]
pub struct PackageFilesQuery<'w, 's> {
	packages: AncestorQuery<'w, 's, &'static PackageFiles>,
	parents: Query<'w, 's, &'static ChildOf>,
	stores: Query<'w, 's, &'static BlobStore>,
}

impl PackageFilesQuery<'_, '_> {
	/// The file `document` names for `entity`.
	///
	/// ## Errors
	/// Errors when nothing above `entity` holds a store.
	pub fn resolve(&self, entity: Entity, document: &DocPath) -> Result<Blob> {
		self.get(entity)?.resolve(document)
	}

	/// The package of `entity`, for a tree built apart from it to carry.
	///
	/// ## Errors
	/// Errors when nothing above `entity` holds a store.
	pub fn get(&self, entity: Entity) -> Result<PackageFiles> {
		match self.packages.get(entity) {
			Ok(package) => package.clone().xok(),
			Err(_) => core::iter::once(entity)
				.chain(self.parents.iter_ancestors(entity))
				.filter_map(|entity| self.stores.get(entity).ok())
				.last()
				.cloned()
				.map(PackageFiles::new)
				.ok_or_else(|| {
					bevyhow!(
						"no store above {entity} to read its documents from: \
						an entry load composes one on its root"
					)
				}),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A [`DocPath`] names a file of the package whatever store is nearer, and
	/// `path_of` answers the path it was named by.
	#[beet_core::test]
	fn resolves_in_the_package_not_the_nearest_store() {
		let mut world = World::new();
		let store = BlobStore::temp();
		let entry = fwd_path!("app/main.bsx");
		let root = world
			.spawn((PackageFiles::of_entry(&store, &entry), store.clone()))
			.id();
		let scoped = world
			.spawn((
				ChildOf(root),
				store.with_subdir(fwd_path!("elsewhere")),
			))
			.id();
		let document = DocPath::parse("widgets/Card.bsx").unwrap();
		world.with_state::<PackageFilesQuery, _>(|packages| {
			let blob = packages.resolve(scoped, &document).unwrap();
			blob.store().subdir().xpect_eq(fwd_path!("app"));
			packages
				.get(scoped)
				.unwrap()
				.path_of(&store.blob(fwd_path!("app/widgets/Card.bsx")))
				.xpect_eq(document.fwd_path().unwrap());
		});
	}
}
