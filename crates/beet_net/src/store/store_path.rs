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
//! A [`DirPath`] resolves against the nearest *ancestor* store (exclusive of
//! self): it produces a store on its own entity, so resolving inclusively would
//! re-scope its own output. A [`BlobPath`] resolves inclusively, so a file
//! declared beside its store (`<DocumentBlob path=".." {FsStore{..}}>`) reads
//! from it. A change-detection pair keeps the produced components
//! correct as [`BlobStore`]s are inserted/removed above them: [`on_insert_store`]
//! re-resolves descendants when an ancestor store appears, and [`on_remove_store`]
//! drops the produced component when its backing store goes away. Both touch
//! descendants only, never the store entity itself, so the cascade is re-entrancy
//! safe. [`on_insert_child_of`] covers the third way an ancestor store changes,
//! the entity (or a subtree) being parented under one after it spawned.

use crate::prelude::*;
use beet_core::prelude::*;

/// Scopes the nearest ancestor [`BlobStore`] to a subdirectory, inserting the scoped
/// store on the same entity (which then backs that entity's descendants).
///
/// The markup-spawnable "serve/read this subtree of the repo store", eg a
/// `ServeBlobs` route paired with `DirPath("assets")` to serve the store's
/// `assets/` subdir.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct DirPath(pub RelPath);

impl DirPath {
	/// The `on_insert` hook body for a component declaring a `src` dir
	/// (`<RoutesDir src="routes"/>`): derive this scope onto its entity, so the
	/// component's store and [`WatchDir`] resolve like any other [`DirPath`].
	///
	/// `#[component(on_insert = hook_ext::component_hook(|dir: &RoutesDir| DirPath::derive(&dir.src)))]`
	pub fn derive(src: &RelPath) -> impl FnOnce(&mut EntityCommands) + use<> {
		let src = src.clone();
		move |entity| {
			entity.insert(DirPath(src));
		}
	}
}

/// Resolves a single [`Blob`] in the nearest ancestor [`BlobStore`], inserting it on
/// the same entity: the "this one file in the store" surface. The produced
/// [`Blob`] is marked `Changed` whenever the object changes, so a consumer keyed
/// on `Changed<Blob>` reads the file on arrival, on re-resolution and on every
/// external edit alike.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
pub struct BlobPath(pub RelPath);

impl BlobPath {
	/// The `on_insert` hook body for a component declaring a file `path`
	/// (`<DocumentBlob path="todos.json"/>`): derive this blob onto its entity,
	/// so the component reads through a [`Blob`] like any other [`BlobPath`].
	///
	/// `#[component(on_insert = hook_ext::component_hook(|doc: &DocumentBlob| BlobPath::derive(&doc.path)))]`
	pub fn derive(path: &RelPath) -> impl FnOnce(&mut EntityCommands) + use<> {
		let path = path.clone();
		move |entity| {
			entity.insert(BlobPath(path));
		}
	}
}

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
	if stores
		.get(entity)
		.is_ok_and(|current| current.same_scope(&scoped))
	{
		return;
	}
	// watch the scoped subdir for live reload (keyed to its base store), so a
	// `RoutesDir`/`TemplateDir`/`AssetsDir` mount's dir reloads; inert on a non-fs
	// store / on wasm. `WatchDir` is notify-backed and std-only, so a no_std target
	// just mounts the scoped store with no live-reload watcher.
	#[cfg(feature = "std")]
	let watch = WatchDir::from_store(&scoped);
	let mut entity_commands = commands.entity(entity);
	entity_commands.insert(scoped);
	#[cfg(feature = "std")]
	if let Some(watch) = watch {
		entity_commands.insert(watch);
	}
}

/// (Re)compute a [`BlobPath`] entity's [`Blob`] from its own or nearest ancestor
/// store, inserting it only when the target changes.
fn resolve_blob_path(
	entity: Entity,
	blob_paths: &Query<&BlobPath>,
	parents: &Query<&ChildOf>,
	stores: &Query<&BlobStore>,
	blobs: &Query<&Blob>,
	commands: &mut Commands,
) {
	let Ok(blob_path) = blob_paths.get(entity) else {
		return;
	};
	let Some(store) = stores.get(entity).ok().or_else(|| {
		nearest_store(entity, parents, stores).map(|(_, store)| store)
	}) else {
		return;
	};
	let blob = store.blob(blob_path.0.clone());
	if blobs
		.get(entity)
		.is_ok_and(|current| current.same_target(&blob))
	{
		return;
	}
	commands.entity(entity).insert(blob);
}

/// On [`DirPath`] insert, scope the nearest ancestor store onto the entity.
pub(crate) fn on_insert_dir_path(
	ev: On<Insert, DirPath>,
	dirs: Query<&DirPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	mut commands: Commands,
) {
	resolve_dir_path(ev.entity, &dirs, &parents, &stores, &mut commands);
}

/// On [`BlobPath`] insert, resolve the [`Blob`] from the nearest ancestor store.
pub(crate) fn on_insert_blob_path(
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

/// (Re)compute every [`DirPath`]/[`BlobPath`] in `entities` against its nearest
/// ancestor store, inserting only where the scope or target changed.
fn resolve_paths(
	entities: impl IntoIterator<Item = Entity>,
	dirs: &Query<&DirPath>,
	blob_paths: &Query<&BlobPath>,
	parents: &Query<&ChildOf>,
	stores: &Query<&BlobStore>,
	blobs: &Query<&Blob>,
	commands: &mut Commands,
) {
	for entity in entities {
		if dirs.contains(entity) {
			resolve_dir_path(entity, dirs, parents, stores, commands);
		} else if blob_paths.contains(entity) {
			resolve_blob_path(
				entity, blob_paths, parents, stores, blobs, commands,
			);
		}
	}
}

/// On [`BlobStore`] insert, re-resolve every descendant [`DirPath`]/[`BlobPath`]
/// against its nearest ancestor store (this entity, or a nearer scoped store),
/// and a [`BlobPath`] on this entity itself. Never a [`DirPath`] on self: the
/// scoped store this fired on is its own output, so re-resolving would compound
/// it.
pub(crate) fn on_insert_store(
	ev: On<Insert, BlobStore>,
	children: Query<&Children>,
	dirs: Query<&DirPath>,
	blob_paths: Query<&BlobPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	blobs: Query<&Blob>,
	mut commands: Commands,
) {
	if blob_paths.contains(ev.entity) {
		resolve_blob_path(
			ev.entity,
			&blob_paths,
			&parents,
			&stores,
			&blobs,
			&mut commands,
		);
	}
	resolve_paths(
		children.iter_descendants(ev.entity),
		&dirs,
		&blob_paths,
		&parents,
		&stores,
		&blobs,
		&mut commands,
	);
}

/// On [`ChildOf`] insert, re-resolve the parented entity and its subtree: a
/// [`DirPath`]/[`BlobPath`] spawned first and parented under a store after
/// (`add_children`, a reparent) resolves like one spawned in place.
pub(crate) fn on_insert_child_of(
	ev: On<Insert, ChildOf>,
	children: Query<&Children>,
	dirs: Query<&DirPath>,
	blob_paths: Query<&BlobPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	blobs: Query<&Blob>,
	mut commands: Commands,
) {
	resolve_paths(
		children.iter_descendants_inclusive(ev.entity),
		&dirs,
		&blob_paths,
		&parents,
		&stores,
		&blobs,
		&mut commands,
	);
}

/// On [`BlobStore`] removal, drop the scoped store / blob it backed on descendants
/// whose nearest store is exactly the one going away (a nearer scoped store backs the
/// rest, and cascades on its own removal). The removed store is still present during
/// this observer, so `nearest_store` still identifies the descendants it backed.
pub(crate) fn on_remove_store(
	ev: On<Remove, BlobStore>,
	children: Query<&Children>,
	dirs: Query<&DirPath>,
	blob_paths: Query<&BlobPath>,
	parents: Query<&ChildOf>,
	stores: Query<&BlobStore>,
	mut commands: Commands,
) {
	// a blob beside the removed store read from it
	if blob_paths.contains(ev.entity) {
		commands.entity(ev.entity).try_remove::<Blob>();
	}
	for descendant in children.iter_descendants(ev.entity) {
		let backed_by_removed = nearest_store(descendant, &parents, &stores)
			.is_some_and(|(holder, _)| holder == ev.entity);
		if !backed_by_removed {
			continue;
		}
		// `try_`: the removal is usually a teardown despawning the whole subtree,
		// so the descendant is routinely gone by the time this command lands.
		if dirs.contains(descendant) {
			// drop the scoped store *and* its watcher registration, so a re-resolve
			// re-registers a `WatchDir` cleanly rather than leaving a stale one.
			// `WatchDir` is std-only; nothing to drop on no_std.
			let mut entity_commands = commands.entity(descendant);
			entity_commands.try_remove::<BlobStore>();
			#[cfg(feature = "std")]
			entity_commands.try_remove::<WatchDir>();
		} else if blob_paths.contains(descendant) {
			commands.entity(descendant).try_remove::<Blob>();
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
			.add_observer(on_insert_child_of)
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
			.spawn((store.clone(), children![DirPath(RelPath::from("assets"))]))
			.id();
		app.update();
		let child = child_of(app.world(), root);
		let scoped = app.world().entity(child).get::<BlobStore>().unwrap();
		scoped.subdir().xpect_eq(RelPath::from("assets"));
		// same backing store, just scoped
		scoped.root_key().xpect_eq(store.root_key());
	}

	/// A [`BlobPath`] under a store resolves to a [`Blob`] in it.
	#[beet_core::test]
	fn blob_path_resolves_blob() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![BlobPath(RelPath::from(
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
			.spawn(children![DirPath(RelPath::from("assets"))])
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
			.xpect_eq(RelPath::from("assets"));
	}

	/// A [`BlobPath`] beside its store resolves from that store, whether the
	/// store spawned with it or arrived on the entity later.
	#[beet_core::test]
	fn blob_path_reads_a_co_located_store() {
		let mut app = store_app();
		let with = app
			.world_mut()
			.spawn((BlobStore::temp(), BlobPath(RelPath::from("a.md"))))
			.id();
		let later = app.world_mut().spawn(BlobPath(RelPath::from("b.md"))).id();
		app.update();
		app.world().entity(with).get::<Blob>().xpect_some();
		app.world().entity(later).get::<Blob>().xpect_none();
		app.world_mut().entity_mut(later).insert(BlobStore::temp());
		app.update();
		app.world().entity(later).get::<Blob>().xpect_some();
		app.world_mut().entity_mut(later).remove::<BlobStore>();
		app.update();
		app.world().entity(later).get::<Blob>().xpect_none();
	}

	/// A [`BlobPath`] spawned on its own and parented under a store afterwards
	/// resolves through the reparent.
	#[beet_core::test]
	fn resolves_when_parented_later() {
		let mut app = store_app();
		let child = app
			.world_mut()
			.spawn(BlobPath(RelPath::from("notes.md")))
			.id();
		app.update();
		app.world().entity(child).get::<Blob>().xpect_none();
		app.world_mut()
			.spawn(BlobStore::temp())
			.add_children(&[child]);
		app.update();
		app.world()
			.entity(child)
			.get::<Blob>()
			.unwrap()
			.path()
			.to_string()
			.xpect_eq("notes.md");
	}

	/// Nested [`DirPath`]s compose: the inner store is the ancestor scoped by both
	/// subdirs, proving the cascade (the outer's produced store backs the inner).
	#[beet_core::test]
	fn nested_dir_paths_compose() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![(
				DirPath(RelPath::from("a")),
				children![DirPath(RelPath::from("b"))]
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
			.xpect_eq(RelPath::from("a/b"));
	}

	/// Removing the backing store drops the scoped store a [`DirPath`] produced.
	#[beet_core::test]
	fn remove_store_drops_scoped() {
		let mut app = store_app();
		let root = app
			.world_mut()
			.spawn((BlobStore::temp(), children![DirPath(RelPath::from(
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
			.spawn((BlobStore::temp(), children![DirPath(RelPath::from(
				"assets"
			))]))
			.id();
		app.update();
		app.world_mut().entity_mut(root).despawn();
		app.update();
		app.world().get_entity(root).is_err().xpect_true();
	}
}
