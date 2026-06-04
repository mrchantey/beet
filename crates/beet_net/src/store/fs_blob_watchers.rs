use crate::prelude::*;
use beet_core::exports::notify::EventKind;
use beet_core::prelude::*;

/// Native filesystem watchers backing reactive [`FsStore`]s.
///
/// One [`FsWatcher`] entity per base `path` (ie per [`root_key`](BlobStoreProvider::root_key)),
/// refcounted by the member entities sharing that path. The watcher emits events
/// **relative to the base path** into the [`BlobEventBus`], so every subdir-derived
/// store routes via its own [`did_change`](BlobStoreProvider::did_change).
#[derive(Default, Resource)]
pub struct FsBlobWatchers {
	/// Per base path: the watcher entity and its subscriber refcount.
	watchers: HashMap<AbsPathBuf, FsWatcherEntry>,
}

/// A single base-path watcher and its refcount.
struct FsWatcherEntry {
	/// The internal [`FsWatcher`] entity, despawned at zero refcount.
	entity: Entity,
	/// Number of member [`FsStore`]s sharing this base path.
	count: usize,
}

/// Ensure the added [`FsStore`]'s base path is covered by exactly one watcher,
/// bumping the refcount if it already exists.
pub fn add_fs_store_watcher(
	ev: On<Add, FsStore>,
	mut commands: Commands,
	mut watchers: ResMut<FsBlobWatchers>,
	stores: Query<&FsStore>,
) {
	let Ok(store) = stores.get(ev.entity) else {
		return;
	};
	let path = store.path().clone();
	if let Some(entry) = watchers.watchers.get_mut(&path) {
		entry.count += 1;
		return;
	}
	// spawn an internal FsWatcher on the base path, forwarding DirEvents to the
	// bus relative to that base.
	let base = path.clone();
	let entity = commands
		.spawn(FsWatcher::new(path.clone()))
		.observe_any(move |ev: On<DirEvent>, bus: Res<BlobEventBus>| {
			forward_dir_event(&ev, &base, &bus);
		})
		.id();
	watchers
		.watchers
		.insert(path, FsWatcherEntry { entity, count: 1 });
}

/// Drop the watcher refcount for the removed [`FsStore`], despawning the watcher
/// entity at zero.
pub fn remove_fs_store_watcher(
	ev: On<Remove, FsStore>,
	mut commands: Commands,
	mut watchers: ResMut<FsBlobWatchers>,
	stores: Query<&FsStore>,
) {
	let Ok(store) = stores.get(ev.entity) else {
		return;
	};
	let path = store.path().clone();
	if let Some(entry) = watchers.watchers.get_mut(&path) {
		entry.count -= 1;
		if entry.count == 0 {
			commands.entity(entry.entity).despawn();
			watchers.watchers.remove(&path);
		}
	}
}

impl FsBlobWatchers {
	/// Number of distinct base-path watchers, for tests.
	pub fn len(&self) -> usize { self.watchers.len() }

	/// Whether there are no active watchers.
	pub fn is_empty(&self) -> bool { self.watchers.is_empty() }
}

/// Convert a [`DirEvent`] into per-object [`BlobEvent`]s relative to `base`.
fn forward_dir_event(event: &DirEvent, base: &AbsPathBuf, bus: &BlobEventBus) {
	let store = BlobStore::new(FsStore::new(base.clone()));
	event.iter().for_each(|path_event| {
		// store-relative path, skipping events outside the base
		let Ok(rel) = path_event.path.strip_prefix(base.as_ref()) else {
			return;
		};
		// directories are not objects
		if path_event.path.is_dir() {
			return;
		}
		let kind = match path_event.kind {
			EventKind::Create(_) => BlobEventKind::Created,
			EventKind::Remove(_) => BlobEventKind::Removed,
			_ => BlobEventKind::Changed,
		};
		bus.send(BlobEvent::new(store.clone(), SmolPath::from(rel), kind));
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Two stores sharing a base path dedup to one refcounted watcher.
	#[beet_core::test]
	fn dedups_by_base_path() {
		let mut world = (MinimalPlugins, AsyncPlugin, StorePlugin).into_world();
		let path = AbsPathBuf::new_workspace_rel(
			"target/tests/beet_net/fs_blob_watchers",
		)
		.unwrap();

		// same base path, one via with_subdir
		let a = world.spawn(FsStore::new(path.clone())).id();
		let b = world
			.spawn(FsStore::new(path.clone()).with_subdir("sub"))
			.id();
		world.flush();
		world.resource::<FsBlobWatchers>().len().xpect_eq(1);

		// despawning one leaves the shared watcher alive
		world.entity_mut(a).despawn();
		world.flush();
		world.resource::<FsBlobWatchers>().len().xpect_eq(1);

		// despawning the last drops it
		world.entity_mut(b).despawn();
		world.flush();
		world.resource::<FsBlobWatchers>().is_empty().xpect_true();
	}
}
