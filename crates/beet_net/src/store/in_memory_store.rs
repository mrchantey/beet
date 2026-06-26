use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
#[cfg(feature = "std")]
use bevy::platform::sync::Mutex;
use bevy::platform::sync::RwLock;
use bytes::Bytes;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

impl BlobStore {
	/// Create a pre-created [`InMemoryStore`] store for testing.
	pub fn new_test() -> Self { Self::new(InMemoryStore::new()) }
}

/// Process-global counter assigning a unique `instance_id` per backing store.
static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A store provider using an in-memory hashmap.
///
/// Inner state is `None` when the store has not been created, and `Some(map)`
/// when it exists. An optional `subdir` scopes all operations to a key prefix.
///
/// Spawned as a [`Component`] (its `on_add` inserts a [`BlobStore`]), it becomes
/// reactive: `insert` / `remove` emit a [`BlobEvent`] on the subscribed bus, and
/// every clone (including [`with_subdir`](BlobStoreProvider::with_subdir)) shares
/// one backing `Arc`, so the `instance_id` and bus subscription are shared.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = BlobStore::on_add::<Self>)]
pub struct InMemoryStore {
	/// Shared backing state, opaque to reflection.
	#[reflect(ignore)]
	inner: Arc<InMemoryInner>,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<SmolPath>,
}

/// Backing state shared across all clones of an [`InMemoryStore`].
#[derive(Debug)]
struct InMemoryInner {
	/// Shared storage state, `None` until the store is created.
	map: RwLock<Option<HashMap<SmolPath, Bytes>>>,
	/// Stable identity used by `root_key`, unique per `new` / `new_empty`.
	instance_id: usize,
	/// Bus to emit [`BlobEvent`]s on, set while at least one watcher subscribes.
	#[cfg(feature = "std")]
	bus: Mutex<Option<async_channel::Sender<BlobEvent>>>,
	/// Number of watcher subscribers; the bus is set on the first, cleared on
	/// the last.
	#[cfg(feature = "std")]
	subscribers: AtomicUsize,
}

impl Default for InMemoryStore {
	fn default() -> Self { Self::new() }
}

impl Default for InMemoryInner {
	/// A fresh empty backing with a new `instance_id`, ie for reflection's
	/// ignored field. Constructing a store this way is never reactive until
	/// `subscribe` runs.
	fn default() -> Self {
		InMemoryInner {
			map: RwLock::new(None),
			instance_id: INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed),
			#[cfg(feature = "std")]
			bus: Mutex::new(None),
			#[cfg(feature = "std")]
			subscribers: AtomicUsize::new(0),
		}
	}
}

impl InMemoryStore {
	/// Creates a new already-created (empty) in-memory provider.
	pub fn new() -> Self { Self::with_map(Some(HashMap::new())) }

	/// Creates a new uncreated in-memory provider.
	pub fn new_empty() -> Self { Self::with_map(None) }

	/// Creates a new already-created provider seeded with `entries`, so a crate
	/// can ship compile-time bytes (eg `include_str!`) into a store synchronously
	/// without an async `insert`. The store is then read like any other (eg a
	/// [`TemplateDir`](beet_core::prelude::TemplateDir) over it).
	pub fn new_seeded(
		entries: impl IntoIterator<Item = (SmolPath, Bytes)>,
	) -> Self {
		Self::with_map(Some(entries.into_iter().collect()))
	}

	/// Build a store with the given initial map and a fresh `instance_id`.
	fn with_map(map: Option<HashMap<SmolPath, Bytes>>) -> Self {
		Self {
			inner: Arc::new(InMemoryInner {
				map: RwLock::new(map),
				instance_id: INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed),
				#[cfg(feature = "std")]
				bus: Mutex::new(None),
				#[cfg(feature = "std")]
				subscribers: AtomicUsize::new(0),
			}),
			subdir: None,
		}
	}

	/// Resolve an external path to the internal key by prepending the subdir.
	fn resolve_key(&self, path: &SmolPath) -> SmolPath {
		match &self.subdir {
			Some(sub) => sub.join(path),
			None => path.clone(),
		}
	}

	/// Subscribe a watcher, setting the bus on the first subscriber.
	#[cfg(feature = "std")]
	pub(crate) fn subscribe(&self, sender: async_channel::Sender<BlobEvent>) {
		if self.inner.subscribers.fetch_add(1, Ordering::SeqCst) == 0 {
			*self.inner.bus.lock().unwrap() = Some(sender);
		}
	}

	/// Unsubscribe a watcher, clearing the bus on the last subscriber.
	#[cfg(feature = "std")]
	pub(crate) fn unsubscribe(&self) {
		if self.inner.subscribers.fetch_sub(1, Ordering::SeqCst) == 1 {
			*self.inner.bus.lock().unwrap() = None;
		}
	}

	/// Emit a [`BlobEvent`] built from `self` if a bus is subscribed.
	#[cfg(feature = "std")]
	fn emit(&self, path: &SmolPath, kind: BlobEventKind) {
		if let Some(sender) = self.inner.bus.lock().unwrap().as_ref() {
			let event = BlobEvent::new(
				BlobStore::new(self.clone()),
				path.clone(),
				kind,
			);
			sender.try_send(event).ok();
		}
	}
}

#[cfg(all(feature = "json", feature = "std"))]
impl<T: TableStoreRow> TableProvider<T> for InMemoryStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BlobStoreProvider for InMemoryStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		Box::new(InMemoryStore {
			inner: self.inner.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn id(&self) -> &'static str { "memory" }

	fn root_key(&self) -> SmolStr {
		format!("memory:{}", self.inner.instance_id).into()
	}

	fn subdir(&self) -> SmolPath { self.subdir.clone().unwrap_or_default() }

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move { this.inner.map.read().unwrap().is_some().xok() })
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.inner.map.write().unwrap();
			if guard.is_some() {
				bevybail!("store already exists")
			}
			*guard = Some(HashMap::new());
			Ok(())
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.inner.map.write().unwrap();
			if guard.is_none() {
				bevybail!("store does not exist")
			}
			*guard = None;
			Ok(())
		})
	}

	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		#[cfg(feature = "std")]
		let path = path.clone();
		Box::pin(async move {
			let existed = {
				let mut guard = this.inner.map.write().unwrap();
				let map = guard
					.as_mut()
					.ok_or_else(|| bevyhow!("store not created"))?;
				map.insert(key, body).is_some()
			};
			#[cfg(feature = "std")]
			this.emit(&path, match existed {
				true => BlobEventKind::Changed,
				false => BlobEventKind::Created,
			});
			let _ = existed;
			Ok(())
		})
	}

	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.map.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.contains_key(&key).xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		let this = self.clone();
		Box::pin(async move {
			let guard = this.inner.map.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			match &this.subdir {
				Some(sub) => {
					// filter keys by prefix and strip it
					map.keys()
						.filter_map(|key| {
							key.as_str()
								.strip_prefix(sub.as_str())
								.map(SmolPath::new)
						})
						.collect::<Vec<_>>()
						.xok()
				}
				None => map.keys().cloned().collect::<Vec<_>>().xok(),
			}
		})
	}

	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.map.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.get(&key)
				.cloned()
				.ok_or_else(|| bevyhow!("object not found: {key}"))
		})
	}

	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		Box::pin(async move {
			{
				let mut guard = this.inner.map.write().unwrap();
				let map = guard
					.as_mut()
					.ok_or_else(|| bevyhow!("store not created"))?;
				map.remove(&key)
					.ok_or_else(|| bevyhow!("object not found: {key}"))?;
			}
			#[cfg(feature = "std")]
			this.emit(&path, BlobEventKind::Removed);
			let _ = path;
			Ok(())
		})
	}

	fn public_url(
		&self,
		_path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { None.xok() })
	}
}

/// Subscribe the added [`InMemoryStore`] to the [`BlobEventBus`], so its
/// `insert` / `remove` emit [`BlobEvent`]s. Refcounted per backing `Arc`, so a
/// `with_subdir` clone adds no duplicate sends.
#[cfg(feature = "std")]
pub fn add_memory_store_watcher(
	ev: On<Add, InMemoryStore>,
	bus: Res<BlobEventBus>,
	stores: Query<&InMemoryStore>,
) {
	if let Ok(store) = stores.get(ev.entity) {
		store.subscribe(bus.sender.clone());
	}
}

/// Unsubscribe the removed [`InMemoryStore`], clearing the bus on the last
/// subscriber.
#[cfg(feature = "std")]
pub fn remove_memory_store_watcher(
	ev: On<Remove, InMemoryStore>,
	stores: Query<&InMemoryStore>,
) {
	if let Ok(store) = stores.get(ev.entity) {
		store.unsubscribe();
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		let provider = InMemoryStore::new_empty();
		store_test::run(provider).await;
	}

	#[beet_core::test]
	fn distinct_instances_have_distinct_root_keys() {
		let a = InMemoryStore::new();
		let b = InMemoryStore::new();
		(a.root_key() != b.root_key()).xpect_true();
		// a subdir clone shares the backing instance
		a.with_subdir(SmolPath::new("sub"))
			.root_key()
			.xpect_eq(a.root_key());
	}
}
