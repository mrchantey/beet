//! Store-path components that resolve a scoped [`BlobStore`] / [`Blob`] from the
//! nearest ancestor store at insert time, so consumers read a ready component
//! instead of walking the ancestor path and re-scoping per call.
//!
//! The author declares intent by which component they use:
//! - [`DirPath`] scopes the nearest ancestor store to a subdirectory, inserting the
//!   scoped [`BlobStore`] on the same entity. That scoped store then becomes the
//!   ancestor store for the entity's descendants, so nested [`DirPath`]/[`BlobPath`]
//!   resolve through it.
//! - [`BlobPath`] resolves a single [`Blob`] in the nearest ancestor store.
//!
//! Resolution is always against the nearest *ancestor* store (exclusive of self): a
//! [`DirPath`] produces a store on its own entity, so resolving inclusively would
//! re-scope its own output. A change-detection pair keeps the produced components
//! correct as [`BlobStore`]s are inserted/removed above them: [`on_insert_store`]
//! re-resolves descendants when an ancestor store appears, and [`on_remove_store`]
//! drops the produced component when its backing store goes away. Both touch
//! descendants only, never the store entity itself, so the cascade is re-entrancy
//! safe.

use crate::prelude::*;
use beet_core::prelude::*;

/// Scopes the nearest ancestor [`BlobStore`] to a subdirectory, inserting the scoped
/// store on the same entity (which then backs that entity's descendants).
///
/// The markup-spawnable "serve/read this subtree of the site store", eg a
/// [`BlobStoreRoute`](crate::prelude) paired with `DirPath("assets")` to serve the
/// store's `assets/` subdir.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct DirPath(pub SmolPath);

/// Resolves a single [`Blob`] in the nearest ancestor [`BlobStore`], inserting it on
/// the same entity: the "this one file in the store" surface.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct BlobPath(pub SmolPath);

/// The nearest *ancestor* [`BlobStore`] (exclusive of `entity`) and the entity that
/// holds it: the parent store a [`DirPath`]/[`BlobPath`] resolves against. Exclusive
/// so a re-resolution starts from the ancestor and never compounds the store this
/// entity itself produced.
fn nearest_store<'a>(
	entity: Entity,
	parents: &Query<&ChildOf>,
	stores: &'a Query<&BlobStore>,
) -> Option<(Entity, &'a BlobStore)> {
	parents.iter_ancestors(entity).find_map(|ancestor| {
		stores.get(ancestor).ok().map(|store| (ancestor, store))
	})
}

/// (Re)compute a [`DirPath`] entity's scoped store from its nearest ancestor store,
/// inserting it only when the scope changes so a cascade of ancestor inserts settles.
fn resolve_dir_path(
	entity: Entity,
	dirs: &Query<&DirPath>,
	parents: &Query<&ChildOf>,
	stores: &Query<&BlobStore>,
	commands: &mut Commands,
) {
	let Ok(dir) = dirs.get(entity) else { return };
	let Some((_, store)) = nearest_store(entity, parents, stores) else {
		return;
	};
	let scoped = store.with_subdir(dir.0.clone());
	if stores.get(entity).is_ok_and(|current| current.same_scope(&scoped)) {
		return;
	}
	commands.entity(entity).insert(scoped);
}

/// (Re)compute a [`BlobPath`] entity's [`Blob`] from its nearest ancestor store,
/// inserting it only when the target changes.
fn resolve_blob_path(
	entity: Entity,
	blob_paths: &Query<&BlobPath>,
	parents: &Query<&ChildOf>,
	stores: &Query<&BlobStore>,
	blobs: &Query<&Blob>,
	commands: &mut Commands,
) {
	let Ok(blob_path) = blob_paths.get(entity) else { return };
	let Some((_, store)) = nearest_store(entity, parents, stores) else {
		return;
	};
	let blob = store.blob(blob_path.0.clone());
	if blobs.get(entity).is_ok_and(|current| current.same_target(&blob)) {
		return;
	}
	commands.entity(entity).insert(blob);
}

/// On [`DirPath`] insert, scope the nearest ancestor store onto the entity.
pub fn on_insert_dir_path(
	ev: On<Insert, DirPath>,
	dirs: Query<&DirPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	mut commands: Commands,
) {
	resolve_dir_path(ev.entity, &dirs, &parents, &stores, &mut commands);
}

/// On [`BlobPath`] insert, resolve the [`Blob`] from the nearest ancestor store.
pub fn on_insert_blob_path(
	ev: On<Insert, BlobPath>,
	blob_paths: Query<&BlobPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	blobs: Query<&Blob>,
	mut commands: Commands,
) {
	resolve_blob_path(
		ev.entity,
		&blob_paths,
		&parents,
		&stores,
		&blobs,
		&mut commands,
	);
}

/// On [`BlobStore`] insert, re-resolve every descendant [`DirPath`]/[`BlobPath`]
/// against its nearest ancestor store (this entity, or a nearer scoped store).
/// Descendants only, never self: the scoped store this fired on is a [`DirPath`]'s
/// own output, so re-resolving self would compound it.
pub fn on_insert_store(
	ev: On<Insert, BlobStore>,
	children: Query<&Children>,
	dirs: Query<&DirPath>,
	blob_paths: Query<&BlobPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	blobs: Query<&Blob>,
	mut commands: Commands,
) {
	for descendant in children.iter_descendants(ev.entity) {
		if dirs.contains(descendant) {
			resolve_dir_path(descendant, &dirs, &parents, &stores, &mut commands);
		} else if blob_paths.contains(descendant) {
			resolve_blob_path(
				descendant,
				&blob_paths,
				&parents,
				&stores,
				&blobs,
				&mut commands,
			);
		}
	}
}

/// On [`BlobStore`] removal, drop the scoped store / blob it backed on descendants
/// whose nearest store is exactly the one going away (a nearer scoped store backs the
/// rest, and cascades on its own removal). The removed store is still present during
/// this observer, so `nearest_store` still identifies the descendants it backed.
pub fn on_remove_store(
	ev: On<Remove, BlobStore>,
	children: Query<&Children>,
	dirs: Query<&DirPath>,
	blob_paths: Query<&BlobPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	mut commands: Commands,
) {
	for descendant in children.iter_descendants(ev.entity) {
		let backed_by_removed = nearest_store(descendant, &parents, &stores)
			.is_some_and(|(holder, _)| holder == ev.entity);
		if !backed_by_removed {
			continue;
		}
		if dirs.contains(descendant) {
			commands.entity(descendant).remove::<BlobStore>();
		} else if blob_paths.contains(descendant) {
			commands.entity(descendant).remove::<Blob>();
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A world whose observers are the store-path resolution pair under test.
	fn store_app() -> App {
		let mut app = App::new();
		app.add_observer(on_insert_dir_path)
			.add_observer(on_insert_blob_path)
			.add_observer(on_insert_store)
			.add_observer(on_remove_store);
		app
	}

	/// First child of `entity`.
	fn child_of(world: &World, entity: Entity) -> Entity {
		world.entity(entity).get::<Children>().unwrap()[0]
	}

	/// A [`DirPath`] under a store resolves to a subdir-scoped store sharing the
	/// ancestor's backing.
	#[beet_core::test]
	fn dir_path_scopes_ancestor_store() {
		let mut app = store_app();
		let store = BlobStore::temp();
		let root = app
			.world_mut()
			.spawn((store.clone(), children![DirPath(SmolPath::from("assets"))]))
			.id();
		app.update();
		let child = child_of(app.world(), root);
		let scoped = app.world().entity(child).get::<BlobStore>().unwrap();
		scoped.subdir().xpect_eq(SmolPath::from("assets"));
		// same backing store, just scoped
		scoped.root_key().xpect_eq(store.root_key());
	}

	/// A [`BlobPath`] under a store resolves to a [`Blob`] in it.
	#[beet_core::test]
	fn blob_path_resolves_blob() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![BlobPath(SmolPath::from(
				"notes.md"
			))]))
			.id();
		app.update();
		let child = child_of(app.world(), root);
		let blob = app.world().entity(child).get::<Blob>().unwrap();
		blob.path().to_string().xpect_eq("notes.md");
	}

	/// A [`DirPath`] whose ancestor store arrives later still resolves: the
	/// [`on_insert_store`] churn re-resolves descendants.
	#[beet_core::test]
	fn resolves_when_store_arrives_later() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn(children![DirPath(SmolPath::from("assets"))])
			.id();
		app.update();
		let child = child_of(app.world(), root);
		// no ancestor store yet, so nothing produced
		app.world().entity(child).get::<BlobStore>().xpect_none();
		// the store appears above it
		app.world_mut().entity_mut(root).insert(BlobStore::temp());
		app.update();
		app.world()
			.entity(child)
			.get::<BlobStore>()
			.unwrap()
			.subdir()
			.xpect_eq(SmolPath::from("assets"));
	}

	/// Nested [`DirPath`]s compose: the inner store is the ancestor scoped by both
	/// subdirs, proving the cascade (the outer's produced store backs the inner).
	#[beet_core::test]
	fn nested_dir_paths_compose() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![(
				DirPath(SmolPath::from("a")),
				children![DirPath(SmolPath::from("b"))]
			)]))
			.id();
		app.update();
		let outer = child_of(app.world(), root);
		let inner = child_of(app.world(), outer);
		app.world()
			.entity(inner)
			.get::<BlobStore>()
			.unwrap()
			.subdir()
			.xpect_eq(SmolPath::from("a/b"));
	}

	/// Removing the backing store drops the scoped store a [`DirPath`] produced.
	#[beet_core::test]
	fn remove_store_drops_scoped() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![DirPath(SmolPath::from(
				"assets"
			))]))
			.id();
		app.update();
		let child = child_of(app.world(), root);
		app.world().entity(child).get::<BlobStore>().xpect_some();
		app.world_mut().entity_mut(root).remove::<BlobStore>();
		app.update();
		app.world().entity(child).get::<BlobStore>().xpect_none();
	}

	/// Despawning a store subtree does not panic the removal cascade.
	#[beet_core::test]
	fn despawn_is_safe() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![DirPath(SmolPath::from(
				"assets"
			))]))
			.id();
		app.update();
		app.world_mut().entity_mut(root).despawn();
		app.update();
		app.world().get_entity(root).is_err().xpect_true();
	}
}
