//! [`InMemoryBlobStore`]: blobs in process memory.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::LazyLock;
use bevy::platform::sync::Mutex;
use bevy::platform::sync::RwLock;
use bevy::platform::sync::Weak;
use bevy::reflect::ReflectRef;
use bytes::Bytes;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

/// Process-global counter minting a unique name per unnamed backing.
static INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Process-global registry of live backings by name, so every handle opened on
/// one name shares one backing. `Weak`, so a store with no live handle is freed
/// and the registry never pins data.
static REGISTRY: LazyLock<Mutex<HashMap<SmolStr, Weak<InMemoryInner>>>> =
	LazyLock::new(default);

/// Blobs in process memory, `memory://<name>[/<path_prefix>]`: for a test, or
/// a runtime whose bytes need not outlive it.
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let store = BlobStore::new(InMemoryBlobStore::new());
/// store.insert(&RelPath::from("hello.txt"), "world").await?;
/// # Ok(())
/// # }
/// ```
///
/// Every backing has a name: [`named`](Self::named) joins the backing of that
/// name, creating it on the first call, so a store a test seeds by name is the
/// store a uri names, and a scene round-trips a memory store onto its data.
/// [`new`](Self::new) mints a name nobody else knows, so the default stays
/// isolated. The backing lives as long as any handle does.
///
/// Inner state is `None` when the store has not been created, and `Some(map)`
/// when it exists. An optional `subdir` scopes all operations to a key prefix.
///
/// Spawned as a [`Component`] (its `on_insert` inserts a [`BlobStore`]), it becomes
/// reactive: `insert` / `remove` emit a [`BlobEvent`] on the subscribed bus, and
/// every handle on one name (including [`with_subdir`](BlobProvider::with_subdir))
/// shares one backing `Arc`, so the bus subscription is shared.
#[derive(Debug, Clone, Get, Component, Reflect)]
// `inner` is rebuilt from the registry by name, see the manual `FromReflect`.
#[reflect(Component, from_reflect = false, FromReflect)]
#[component(on_insert = BlobStore::on_insert::<Self>)]
pub struct InMemoryBlobStore {
	/// The backing's name, the `<name>` in `memory://<name>`.
	name: SmolStr,
	/// Shared backing state, opaque to reflection.
	#[reflect(ignore)]
	#[get(skip)]
	inner: Arc<InMemoryInner>,
	/// Optional subdirectory prefix for all keys.
	#[get(skip)]
	subdir: Option<RelPath>,
}

/// Backing state shared across every handle on one name.
#[derive(Debug)]
struct InMemoryInner {
	/// Shared storage state, `None` until the store is created.
	map: RwLock<Option<HashMap<RelPath, Bytes>>>,
	/// Bus to emit [`BlobEvent`]s on, set while at least one watcher subscribes.
	#[cfg(feature = "std")]
	bus: Mutex<Option<async_channel::Sender<BlobEvent>>>,
	/// Number of watcher subscribers; the bus is set on the first, cleared on
	/// the last.
	#[cfg(feature = "std")]
	subscribers: AtomicUsize,
}

impl InMemoryInner {
	fn new(map: Option<HashMap<RelPath, Bytes>>) -> Self {
		Self {
			map: RwLock::new(map),
			#[cfg(feature = "std")]
			bus: Mutex::new(None),
			#[cfg(feature = "std")]
			subscribers: AtomicUsize::new(0),
		}
	}
}

impl Drop for InMemoryInner {
	/// The last handle dropped: prune the registry so a dead name is freed
	/// and a later [`InMemoryBlobStore::named`] starts fresh.
	fn drop(&mut self) {
		if let Ok(mut registry) = REGISTRY.lock() {
			registry.retain(|_, weak| weak.strong_count() > 0);
		}
	}
}

impl Default for InMemoryBlobStore {
	fn default() -> Self { Self::new() }
}

/// Rebuild from the registry by name, so a scene round-trips a memory store
/// onto its data: the derived impl would default the ignored `inner` to a
/// fresh backing. A concrete value (a reflect clone) downcasts directly.
impl FromReflect for InMemoryBlobStore {
	fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
		if let Some(value) = reflect.try_downcast_ref::<Self>() {
			return Some(value.clone());
		}
		let ReflectRef::Struct(dyn_struct) = reflect.reflect_ref() else {
			return None;
		};
		let name = SmolStr::from_reflect(dyn_struct.field("name")?)?;
		let subdir = dyn_struct
			.field("subdir")
			.and_then(Option::<RelPath>::from_reflect)
			.flatten();
		let mut store = Self::named(name);
		store.subdir = subdir;
		Some(store)
	}
}

impl InMemoryBlobStore {
	/// Creates a new already-created (empty) in-memory provider under a minted
	/// name, isolated because nobody else knows it.
	pub fn new() -> Self { Self::minted(Some(HashMap::new())) }

	/// Creates a new uncreated in-memory provider under a minted name.
	pub fn new_empty() -> Self { Self::minted(None) }

	/// Creates a new already-created provider seeded with `entries`, so a crate
	/// can ship compile-time bytes (eg `include_str!`) into a store synchronously
	/// without an async `insert`. The store is then read like any other.
	pub fn new_seeded(
		entries: impl IntoIterator<Item = (RelPath, Bytes)>,
	) -> Self {
		Self::minted(Some(entries.into_iter().collect()))
	}

	/// The store named `name`: joins the live backing of that name, else creates
	/// an already-created (empty) one. Two handles on one name see each other's
	/// writes for as long as either lives.
	pub fn named(name: impl Into<SmolStr>) -> Self {
		let name = name.into();
		let mut registry = REGISTRY.lock().unwrap();
		match registry.get(&name).and_then(Weak::upgrade) {
			Some(inner) => Self {
				name,
				inner,
				subdir: None,
			},
			None => Self::register(&mut registry, name, Some(HashMap::new())),
		}
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	/// A fresh backing with initial state `map`, registered under a minted
	/// name no live backing holds (a hand-chosen name may share the spelling).
	fn minted(map: Option<HashMap<RelPath, Bytes>>) -> Self {
		let mut registry = REGISTRY.lock().unwrap();
		let name = loop {
			let name = SmolStr::from(format!(
				"mem-{}",
				INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
			));
			if registry
				.get(&name)
				.is_none_or(|weak| weak.strong_count() == 0)
			{
				break name;
			}
		};
		Self::register(&mut registry, name, map)
	}

	/// A fresh backing with initial state `map`, registered under `name`.
	fn register(
		registry: &mut HashMap<SmolStr, Weak<InMemoryInner>>,
		name: SmolStr,
		map: Option<HashMap<RelPath, Bytes>>,
	) -> Self {
		let inner = Arc::new(InMemoryInner::new(map));
		registry.insert(name.clone(), Arc::downgrade(&inner));
		Self {
			name,
			inner,
			subdir: None,
		}
	}

	/// Resolve an external path to the internal key by prepending the subdir.
	fn resolve_key(&self, path: &RelPath) -> RelPath {
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
	fn emit(&self, path: &RelPath, kind: BlobEventKind) {
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

impl StoreBackend for InMemoryBlobStore {
	fn id(&self) -> &'static str { "memory" }

	fn describe(&self) -> String { self.root_key().to_string() }

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
}

impl BlobProvider for InMemoryBlobStore {
	fn box_clone(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobProvider> {
		Box::new(InMemoryBlobStore {
			name: self.name.clone(),
			inner: self.inner.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn base(&self) -> Box<dyn BlobProvider> {
		Box::new(InMemoryBlobStore {
			subdir: None,
			..self.clone()
		})
	}

	fn root_key(&self) -> SmolStr { format!("memory:{}", self.name).into() }

	fn subdir(&self) -> RelPath { self.subdir.clone().unwrap_or_default() }

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
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

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
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

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let this = self.clone();
		Box::pin(async move {
			let guard = this.inner.map.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			match &this.subdir {
				Some(sub) => {
					// filter keys by prefix, segment-wise so `sub` never
					// matches `subway`, and strip it
					map.keys()
						.filter_map(|key| key.strip_prefix(sub))
						.filter(|key| !key.is_empty())
						.collect::<Vec<_>>()
						.xok()
				}
				None => map.keys().cloned().collect::<Vec<_>>().xok(),
			}
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.map.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			// a miss is a 404, matching every other backend, so a served route
			// distinguishes an absent file from a broken store.
			map.get(&key).cloned().ok_or_else(|| {
				HttpError::new(
					StatusCode::NOT_FOUND,
					format!("object not found: {key}"),
				)
				.into()
			})
		})
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
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

	fn conditional_writes(&self) -> bool { true }

	fn get_versioned(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.map.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.get(&key)
				.map(|bytes| (bytes.clone(), BlobVersion::of(bytes)))
				.xok()
		})
	}

	/// Checked and written under the backing's write lock, the version the
	/// content's digest.
	fn insert_if(
		&self,
		path: &RelPath,
		body: Bytes,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result<BlobVersion>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		Box::pin(async move {
			let version = BlobVersion::of(&body);
			let existed = {
				let mut guard = this.inner.map.write().unwrap();
				let map = guard
					.as_mut()
					.ok_or_else(|| bevyhow!("store not created"))?;
				let found = map.get(&key).map(|bytes| BlobVersion::of(bytes));
				BlobConflict::check(&condition, &path, found.as_ref())?;
				map.insert(key, body).is_some()
			};
			#[cfg(feature = "std")]
			this.emit(&path, match existed {
				true => BlobEventKind::Changed,
				false => BlobEventKind::Created,
			});
			let _ = (existed, path);
			version.xok()
		})
	}

	fn remove_if(
		&self,
		path: &RelPath,
		condition: BlobCondition,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		let path = path.clone();
		Box::pin(async move {
			let removed = {
				let mut guard = this.inner.map.write().unwrap();
				let map = guard
					.as_mut()
					.ok_or_else(|| bevyhow!("store not created"))?;
				let found = map.get(&key).map(|bytes| BlobVersion::of(bytes));
				BlobConflict::check(&condition, &path, found.as_ref())?;
				map.remove(&key).is_some()
			};
			#[cfg(feature = "std")]
			if removed {
				this.emit(&path, BlobEventKind::Removed);
			}
			let _ = (removed, path);
			Ok(())
		})
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { None.xok() })
	}
}

/// Subscribe the added [`InMemoryBlobStore`] to the [`BlobEventBus`], so its
/// `insert` / `remove` emit [`BlobEvent`]s. Refcounted per backing `Arc`, so a
/// `with_subdir` clone adds no duplicate sends.
#[cfg(feature = "std")]
pub(crate) fn add_memory_store_watcher(
	ev: On<Add<InMemoryBlobStore>>,
	bus: Res<BlobEventBus>,
	stores: Query<&InMemoryBlobStore>,
) {
	if let Ok(store) = stores.get(ev.entity) {
		store.subscribe(bus.sender.clone());
	}
}

/// Unsubscribe the removed [`InMemoryBlobStore`], clearing the bus on the last
/// subscriber.
#[cfg(feature = "std")]
pub(crate) fn remove_memory_store_watcher(
	ev: On<Remove<InMemoryBlobStore>>,
	stores: Query<&InMemoryBlobStore>,
) {
	if let Ok(store) = stores.get(ev.entity) {
		store.unsubscribe();
	}
}

#[cfg(test)]
mod test {
	use super::REGISTRY;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bytes::Bytes;

	#[beet_core::test]
	async fn works() {
		let provider = InMemoryBlobStore::new_empty();
		store_test::run(provider).await;
	}

	/// Two handles on one name are one store, and a subdir view keeps the
	/// backing.
	#[beet_core::test]
	async fn handles_by_name_share_a_backing() {
		let first = InMemoryBlobStore::named("shared-backing");
		let second = InMemoryBlobStore::named("shared-backing");
		first.root_key().xpect_eq(second.root_key());
		first.root_key().xpect_eq("memory:shared-backing");
		first
			.insert(&RelPath::new("a.txt"), Bytes::from_static(b"hi"))
			.await
			.unwrap();
		second
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_eq(Bytes::from_static(b"hi"));
		first
			.clone()
			.with_subdir("sub")
			.root_key()
			.xpect_eq(first.root_key());
	}

	/// A minted name is nobody else's: two `new` stores never see each other.
	#[beet_core::test]
	async fn minted_stores_are_isolated() {
		let first = InMemoryBlobStore::new();
		let second = InMemoryBlobStore::new();
		(first.root_key() != second.root_key()).xpect_true();
		first
			.insert(&RelPath::new("a.txt"), Bytes::from_static(b"hi"))
			.await
			.unwrap();
		second
			.exists(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_false();
	}

	/// The registry pins nothing: the entry is gone once the last handle
	/// drops, and the name then starts fresh.
	#[beet_core::test]
	async fn a_dead_name_leaves_the_registry() {
		let registered =
			|name: &str| REGISTRY.lock().unwrap().contains_key(name);
		let store = InMemoryBlobStore::named("ephemeral");
		store
			.insert(&RelPath::new("a.txt"), Bytes::from_static(b"hi"))
			.await
			.unwrap();
		registered("ephemeral").xpect_true();
		drop(store);
		registered("ephemeral").xpect_false();
		InMemoryBlobStore::named("ephemeral")
			.exists(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_false();
	}

	/// The reflected fields rebuild the store onto its live backing, so a
	/// scene round-trips a memory store onto its data.
	#[beet_core::test]
	async fn from_reflect_joins_the_backing() {
		let store = InMemoryBlobStore::named("reflected").with_subdir("docs");
		store
			.insert(&RelPath::new("a.txt"), Bytes::from_static(b"hi"))
			.await
			.unwrap();
		let dynamic = store.to_dynamic().unwrap();
		let rebuilt =
			InMemoryBlobStore::from_reflect(dynamic.as_ref()).unwrap();
		rebuilt.name().xpect_eq("reflected");
		rebuilt.subdir().xpect_eq(RelPath::new("docs"));
		rebuilt
			.get(&RelPath::new("a.txt"))
			.await
			.unwrap()
			.xpect_eq(Bytes::from_static(b"hi"));
	}
}
