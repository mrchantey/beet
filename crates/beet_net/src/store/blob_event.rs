use crate::prelude::*;
use beet_core::prelude::*;

/// A change to a single object in a [`BlobStore`], mirroring the filesystem.
///
/// Emitted by a provider's watcher (fs notify, in-memory broadcast,
/// localStorage `storage`) or by a local write/remove action, drained into a
/// global [`On<BlobEvent>`] and fanned out by [`propagate_blob_store_changes`]
/// / [`propagate_blob_changes`] which mark matching [`BlobStore`] / [`Blob`]
/// components `Changed`.
#[derive(Clone, Event)]
pub struct BlobEvent {
	/// Path of the object relative to [`store`](Self::store), ie usable as
	/// `store.get(&path)`.
	pub path: SmolPath,
	/// Handle to the originating store, so a subscriber can fetch bytes and so
	/// routing can read its [`root_key`](BlobStoreProvider::root_key) /
	/// [`subdir`](BlobStoreProvider::subdir).
	pub store: BlobStore,
	/// What happened to the object.
	pub kind: BlobEventKind,
}

/// The kind of change a [`BlobEvent`] describes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlobEventKind {
	/// The object was created.
	Created,
	/// The object's bytes changed.
	Changed,
	/// The object was removed.
	Removed,
}

impl BlobEvent {
	/// Create a new [`BlobEvent`].
	pub fn new(store: BlobStore, path: SmolPath, kind: BlobEventKind) -> Self {
		Self { path, store, kind }
	}

	/// The event's location relative to its `root_key`, ie
	/// `store.subdir().join(&path)`. This is the routing key compared against a
	/// store's [`subdir`](BlobStoreProvider::subdir).
	pub fn root_relative_path(&self) -> SmolPath {
		self.store.subdir().join(&self.path)
	}
}

/// Separator-aware prefix test: `true` if `key` is `scope` or a child of it.
///
/// An empty `scope` covers all keys. `a/b` never matches `a/bc`.
pub fn key_covers(scope: &SmolPath, key: &SmolPath) -> bool {
	scope.is_empty()
		|| key.as_str() == scope.as_str()
		|| key
			.as_str()
			.strip_prefix(scope.as_str())
			.is_some_and(|rest| rest.starts_with('/'))
}

/// Resource wrapping the global [`BlobEvent`] channel.
///
/// Providers send into the [`Sender`](async_channel::Sender); [`drain_blob_events`]
/// receives and triggers each event each [`PreUpdate`].
#[cfg(feature = "std")]
#[derive(Resource, Clone)]
pub struct BlobEventBus {
	/// Send half handed to provider watchers and write actions.
	pub sender: async_channel::Sender<BlobEvent>,
	receiver: async_channel::Receiver<BlobEvent>,
}

#[cfg(feature = "std")]
impl Default for BlobEventBus {
	fn default() -> Self {
		let (sender, receiver) = async_channel::unbounded();
		Self { sender, receiver }
	}
}

#[cfg(feature = "std")]
impl BlobEventBus {
	/// Send a [`BlobEvent`] into the bus, ignoring a dropped receiver.
	pub fn send(&self, event: BlobEvent) { self.sender.try_send(event).ok(); }
}

/// Drains the [`BlobEventBus`] channel, triggering each [`BlobEvent`] globally.
#[cfg(feature = "std")]
pub fn drain_blob_events(bus: Res<BlobEventBus>, mut commands: Commands) {
	while let Ok(event) = bus.receiver.try_recv() {
		commands.trigger(event);
	}
}

/// Marks each [`BlobStore`] `Changed` when it owns the event scope.
///
/// Listing only changes on `Created` / `Removed`, not on a byte `Changed`.
/// Reading through `Mut`'s `Deref` in the filter does not mark; only the
/// matched `set_changed()` marks, so unrelated stores stay unchanged.
pub fn propagate_blob_store_changes(
	ev: On<BlobEvent>,
	mut stores: Query<&mut BlobStore>,
) {
	if matches!(ev.kind, BlobEventKind::Created | BlobEventKind::Removed) {
		stores
			.iter_mut()
			.filter(|store| store.did_change(&ev))
			.for_each(|mut store| store.set_changed());
	}
}

/// Marks each [`Blob`] `Changed` when the event is its object.
///
/// `Removed` marks the [`Blob`] `Changed` but never despawns it: the handle
/// stays and the object "may not exist", which `get` / `exists` already handle.
pub fn propagate_blob_changes(ev: On<BlobEvent>, mut blobs: Query<&mut Blob>) {
	blobs
		.iter_mut()
		.filter(|blob| blob.matches_event(&ev))
		.for_each(|mut blob| blob.set_changed());
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn key_covers_is_separator_aware() {
		// empty scope covers all
		key_covers(&SmolPath::default(), &SmolPath::new("a/b")).xpect_true();
		// exact match
		key_covers(&SmolPath::new("a/b"), &SmolPath::new("a/b")).xpect_true();
		// child
		key_covers(&SmolPath::new("a"), &SmolPath::new("a/b")).xpect_true();
		// sibling-prefix is not a cover
		key_covers(&SmolPath::new("a/b"), &SmolPath::new("a/bc")).xpect_false();
		// unrelated
		key_covers(&SmolPath::new("a"), &SmolPath::new("b")).xpect_false();
	}

	/// `did_change` / `matches_event` truth table across distinct instances,
	/// nested subdirs (same backing, covering), and a mismatched backing.
	#[beet_core::test]
	fn routing_truth_table() {
		let base = BlobStore::new(InMemoryStore::new());
		let other = BlobStore::new(InMemoryStore::new());
		let sub = base.with_subdir(SmolPath::new("dir"));
		let ev = BlobEvent::new(
			base.clone(),
			SmolPath::new("dir/x.txt"),
			BlobEventKind::Created,
		);

		// base covers the event (root scope covers all)
		base.did_change(&ev).xpect_true();
		// the matching subdir store covers it
		sub.did_change(&ev).xpect_true();
		// a non-covering sibling subdir does not
		base.with_subdir(SmolPath::new("other"))
			.did_change(&ev)
			.xpect_false();
		// a different backing instance never matches
		other.did_change(&ev).xpect_false();

		// blob object-exact match
		base.blob(SmolPath::new("dir/x.txt"))
			.matches_event(&ev)
			.xpect_true();
		base.blob(SmolPath::new("dir/y.txt"))
			.matches_event(&ev)
			.xpect_false();
	}

	/// In-memory insert/remove emits events that mark the owning [`BlobStore`]
	/// `Changed`, and never a sibling store with a different backing.
	#[beet_core::test]
	fn propagates_to_owning_store_only() {
		#[derive(Default, Resource)]
		struct Changes {
			owner: usize,
			sibling: usize,
		}

		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin))
			.init_resource::<Changes>();
		let store = InMemoryStore::new();
		let owner = app.world_mut().spawn(store.clone()).id();
		let sibling = app.world_mut().spawn(InMemoryStore::new()).id();
		app.add_systems(
			Update,
			move |stores: Query<Entity, Changed<BlobStore>>,
			      mut changes: ResMut<Changes>| {
				stores.iter().for_each(|entity| {
					if entity == owner {
						changes.owner += 1;
					} else if entity == sibling {
						changes.sibling += 1;
					}
				});
			},
		);
		// first update subscribes the watcher and clears insertion Changed
		app.update();
		app.world_mut().resource_mut::<Changes>().owner = 0;
		app.world_mut().resource_mut::<Changes>().sibling = 0;

		// insert through the store (immediate + synchronous emit), then the next
		// update drains the bus and propagates
		async_ext::block_on(
			BlobStore::new(store).insert(&SmolPath::new("a.txt"), "a"),
		)
		.unwrap();
		app.update();

		(app.world().resource::<Changes>().owner > 0).xpect_true();
		app.world().resource::<Changes>().sibling.xpect_eq(0);
	}

	/// The in-memory watcher emits a `Created` then a `Removed` [`BlobEvent`] for an
	/// insert then a remove of the same object, drained through the bus into the
	/// global `On<BlobEvent>`. The store-agnostic watcher path live reload rides.
	#[beet_core::test]
	async fn in_memory_watcher_emits_created_then_removed() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, StorePlugin));
		let kinds = Store::<Vec<BlobEventKind>>::default();
		let captor = kinds.clone();
		app.world_mut()
			.add_observer(move |ev: On<BlobEvent>| captor.push(ev.kind));
		// spawning the store subscribes its watcher; the first update flushes it
		let store = InMemoryStore::new();
		app.world_mut().spawn(store.clone());
		app.update();

		let handle = BlobStore::new(store);
		handle.insert(&SmolPath::new("a.txt"), "a").await.unwrap();
		app.update();
		handle.remove(&SmolPath::new("a.txt")).await.unwrap();
		app.update();

		kinds
			.get()
			.xpect_eq(vec![BlobEventKind::Created, BlobEventKind::Removed]);
	}
}
