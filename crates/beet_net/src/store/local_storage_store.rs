use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use js_sys::wasm_bindgen::JsCast;

/// A store provider backed by browser localStorage.
///
/// Uses `store:<store_name>:<path>` as the localStorage key prefix.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = BlobStore::on_add::<Self>)]
pub struct LocalStorageStore {
	/// The store name used as part of the localStorage key prefix.
	store_name: SmolStr,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<SmolPath>,
}

impl LocalStorageStore {
	/// Creates a new localStorage-backed store provider for the given store.
	pub fn new(store_name: impl Into<SmolStr>) -> Self {
		Self {
			store_name: store_name.into(),
			subdir: None,
		}
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<SmolPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	fn store_prefix(&self) -> String {
		format!("store:{}:", self.store_name)
	}

	/// Compose the localStorage key for the store and path.
	fn storage_key(&self, path: &SmolPath) -> String {
		let effective = match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		};
		self.store_prefix().xtend(&effective)
	}

	fn local_storage() -> web_sys::Storage {
		// deno also has localstorage so perform unchecked cast
		js_sys::global()
			.unchecked_into::<web_sys::Window>()
			.local_storage()
			.expect("LocalStorage is not available")
			.expect("Failed to access localStorage")
	}

	/// Create a [`TypedBlob`] handle for a single object in this store.
	pub fn blob(&self, path: SmolPath) -> TypedBlob<Self> {
		TypedBlob::new(self.clone(), path)
	}
}

#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for LocalStorageStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BlobStoreProvider for LocalStorageStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		Box::new(LocalStorageStore {
			store_name: self.store_name.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn id(&self) -> &'static str { "localstorage" }

	fn root_key(&self) -> SmolStr {
		format!("localstorage:{}", self.store_name).into()
	}

	fn subdir(&self) -> SmolPath { self.subdir.clone().unwrap_or_default() }

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let prefix = self.store_prefix();
		Box::pin(async move {
			let storage = Self::local_storage();
			for i in 0..storage.length().unwrap_or(0) {
				if let Some(key) = storage.key(i).ok().flatten() {
					if key.starts_with(&prefix) {
						return true.xok();
					}
				}
			}
			false.xok()
		})
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		// No-op for localStorage
		Box::pin(async { ().xok() })
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let prefix = self.store_prefix();
		Box::pin(async move {
			let storage = Self::local_storage();
			let keys: Vec<String> = (0..storage.length().unwrap_or(0))
				.filter_map(|i| storage.key(i).ok().flatten())
				.filter(|key| key.starts_with(&prefix))
				.collect();
			for key in keys {
				storage.remove_item(&key).ok();
			}
			().xok()
		})
	}

	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		let key = self.storage_key(path);
		let value = BASE64_STANDARD.encode(body);
		Box::pin(async move {
			Self::local_storage().set_item(&key, &value).map_jserr()?;
			().xok()
		})
	}

	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		let key = self.storage_key(path);
		Box::pin(async move {
			let storage = Self::local_storage();
			let value = storage.get_item(&key).map_jserr()?;
			value.is_some().xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		let prefix = self.store_prefix();
		let subdir_prefix = self.subdir.as_ref().map(|s| format!("{}/", s));
		Box::pin(async move {
			let storage = Self::local_storage();
			let keys: Vec<SmolPath> = (0..storage.length().unwrap_or(0))
				.filter_map(|i| storage.key(i).ok().flatten())
				.filter_map(|key| {
					let raw = key.strip_prefix(&prefix)?;
					let rel = match &subdir_prefix {
						Some(p) => raw.strip_prefix(p.as_str())?,
						None => raw,
					};
					Some(SmolPath::new(rel))
				})
				.collect();
			keys.xok()
		})
	}

	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		let key = self.storage_key(path);
		Box::pin(async move {
			let value = Self::local_storage().get_item(&key).map_jserr()?;
			match value {
				Some(val) => {
					// base64 is no_std here, so DecodeError lacks the `Error`
					// impl BevyError's blanket `From` needs; wrap via Display.
					let bytes =
						BASE64_STANDARD.decode(val).map_err(|err| bevyhow!("{err}"))?;
					Bytes::from(bytes).xok()
				}
				None => bevybail!("Object not found: {}", key),
			}
		})
	}

	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		let key = self.storage_key(path);
		let this = self.clone();
		let path = path.clone();
		Box::pin(async move {
			match this.exists(&path).await? {
				true => {
					Self::local_storage().remove_item(&key).map_jserr()?;
					().xok()
				}
				false => {
					bevybail!("Object not found: {}", key)
				}
			}
		})
	}

	fn public_url(
		&self,
		_path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { None.xok() })
	}
}

use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use std::rc::Rc;

/// A single global `storage` event listener forwarding cross-tab localStorage
/// mutations into the [`BlobEventBus`], refcounted across all
/// [`LocalStorageStore`]s.
///
/// `storage` only fires for *other* tabs, so same-tab writes are covered by the
/// write-path [`BlobEvent`] trigger instead. Installed by
/// [`add_local_storage_store_watcher`] on the first store, torn down on the last.
///
/// `!Send` (it owns the JS closure), so it lives in a [`NonSend`](bevy::ecs::system::NonSend)
/// resource.
#[derive(Default)]
pub struct LocalStorageBlobWatcher {
	/// Cleared on the last unsubscribe to stop the forwarding task.
	alive: Option<Rc<AtomicBool>>,
	/// Number of active [`LocalStorageStore`]s.
	subscribers: usize,
}

/// Install the global `storage` listener on the first [`LocalStorageStore`].
pub fn add_local_storage_store_watcher(
	_ev: On<Add, LocalStorageStore>,
	bus: Res<BlobEventBus>,
	spawner: Res<AsyncSpawner>,
	mut watcher: NonSendMut<LocalStorageBlobWatcher>,
) {
	watcher.subscribers += 1;
	if watcher.subscribers > 1 {
		return;
	}
	let alive = Rc::new(AtomicBool::new(true));
	watcher.alive = Some(alive.clone());
	let sender = bus.sender.clone();
	spawner.spawn_local(async move {
		let mut listener =
			HtmlEventListener::<web_sys::StorageEvent>::new("storage");
		while alive.load(Ordering::Relaxed) {
			let Some(ev) = listener.next_event().await else {
				break;
			};
			if let Some(event) = storage_event_to_blob(&ev) {
				sender.try_send(event).ok();
			}
		}
	});
}

/// Drop the refcount, tearing down the listener on the last
/// [`LocalStorageStore`].
pub fn remove_local_storage_store_watcher(
	_ev: On<Remove, LocalStorageStore>,
	mut watcher: NonSendMut<LocalStorageBlobWatcher>,
) {
	watcher.subscribers = watcher.subscribers.saturating_sub(1);
	if watcher.subscribers == 0 {
		if let Some(alive) = watcher.alive.take() {
			alive.store(false, Ordering::Relaxed);
		}
	}
}

/// Map a `storage` event to a [`BlobEvent`], or `None` if its key is not a beet
/// store key. Derives the kind from `oldValue` / `newValue` null-ness.
fn storage_event_to_blob(ev: &web_sys::StorageEvent) -> Option<BlobEvent> {
	let key = ev.key()?;
	// key format: `store:{store_name}:{path}`
	let rest = key.strip_prefix("store:")?;
	let (store_name, path) = rest.split_once(':')?;
	let kind = match (ev.old_value().is_some(), ev.new_value().is_some()) {
		(false, true) => BlobEventKind::Created,
		(true, false) => BlobEventKind::Removed,
		_ => BlobEventKind::Changed,
	};
	let store = BlobStore::new(LocalStorageStore::new(store_name));
	Some(BlobEvent::new(store, SmolPath::new(path), kind))
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[beet_core::test]
	async fn works() {
		let provider = LocalStorageStore::new("test-store");
		store_test::run(provider).await;
	}
}
