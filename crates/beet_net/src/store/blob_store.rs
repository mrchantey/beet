use crate::prelude::*;
use beet_core::prelude::bevy_ecs::error::ErrorContext;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bytes::Bytes;

/// Cross-service storage store for S3, filesystem, memory, or other providers.
///
/// Wraps an [`Arc<dyn BlobStoreProvider>`] and delegates all operations to the
/// inner provider. Implements [`BlobStoreProvider`] itself, so it can be used
/// anywhere a provider is expected.
#[derive(Clone, Component)]
pub struct BlobStore {
	/// Provider that handles store operations (S3, filesystem, memory, etc.)
	provider: Arc<dyn BlobStoreProvider>,
}

impl core::fmt::Debug for BlobStore {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("BlobStore").finish_non_exhaustive()
	}
}

impl BlobStore {
	/// Creates a new store wrapping the given provider.
	pub fn new(provider: impl BlobStoreProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// Creates a new store from a pre-existing [`Arc<dyn BlobStoreProvider>`].
	pub fn from_arc(provider: Arc<dyn BlobStoreProvider>) -> Self {
		Self { provider }
	}

	/// Create temporary in-memory store for testing.
	/// The returned store is pre-created and ready for immediate use.
	pub fn temp() -> BlobStore { BlobStore::new(InMemoryStore::new()) }

	/// Create local store with platform-specific provider.
	/// - wasm: [`LocalStorageStore`]
	/// - native: [`FsStore`] at `.cache/stores/<name>`
	#[cfg(feature = "std")]
	pub fn new_local(name: impl Into<String>) -> BlobStore {
		let name = name.into();
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				BlobStore::new(LocalStorageStore::new(name))
			} else {
				BlobStore::new(FsStore::new(
					AbsPathBuf::new_workspace_rel(format!(".cache/stores/{name}"))
						.unwrap(),
				))
			}
		}
	}

	/// Returns a new store scoped to the given subdirectory.
	pub fn with_subdir(&self, path: SmolPath) -> BlobStore {
		BlobStore::from_arc(Arc::from(self.provider.with_subdir(path)))
	}

	/// Whether `other` resolves the same scope: same backing store and subdir. The
	/// [`DirPath`] change-detection uses it to skip a no-op re-scope, so a cascade of
	/// ancestor [`BlobStore`] inserts settles instead of re-firing forever.
	pub fn same_scope(&self, other: &BlobStore) -> bool {
		self.root_key() == other.root_key() && self.subdir() == other.subdir()
	}

	/// Get an object and infer the [`MediaType`] from its path extension.
	pub async fn get_media(&self, path: &SmolPath) -> Result<MediaBytes> {
		let media_type = path.media_type().unwrap_or(MediaType::Bytes);
		let bytes = self.get(path).await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Create a [`Blob`] handle for a single object in this store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let store = BlobStore::temp();
	/// let blob = store.blob(SmolPath::new("my-file.txt"));
	/// ```
	pub fn blob(&self, path: SmolPath) -> Blob { Blob::new(self.clone(), path) }

	/// Component hook that reads a concrete store component from
	/// the entity and inserts a [`BlobStore`] wrapping it.
	/// Use with `#[component(on_add = BlobStore::on_add::<MyStore>)]`.
	pub fn on_add<T: Component + Clone + BlobStoreProvider>(
		mut world: DeferredWorld,
		cx: HookContext,
	) {
		match world.entity(cx.entity).get_or_else::<T>().cloned() {
			Ok(provider) => {
				world
					.commands()
					.entity(cx.entity)
					.insert(BlobStore::new(provider));
			}
			Err(err) => {
				world.fallback_error_handler()(err, ErrorContext::Command {
					name: core::any::type_name_of_val(&BlobStore::on_add::<T>)
						.into(),
				});
			}
		}
	}

	/// Insert object into store.
	///
	/// Ergonomic wrapper accepting any `impl Into<Bytes>` (eg. `&str`, `Vec<u8>`).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.insert(&SmolPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(
		&self,
		path: &SmolPath,
		body: impl Into<Bytes>,
	) -> Result {
		self.provider.insert(path, body.into()).await
	}

	/// Insert object, failing if it already exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.try_insert(&SmolPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn try_insert(
		&self,
		path: &SmolPath,
		body: impl Into<Bytes>,
	) -> Result {
		if self.exists(path).await? {
			bevybail!("Object already exists: {}", path)
		} else {
			self.insert(path, body).await
		}
	}

	/// Get all objects and their data.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let all_data = store.get_all().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Caution
	/// Expensive operation - prefer [`BlobStoreProvider::list`] + [`BlobStoreProvider::get`]
	pub async fn get_all(&self) -> Result<Vec<(SmolPath, Bytes)>> {
		self.list()
			.await?
			.into_iter()
			.map(async |path| {
				let data = self.get(&path).await?;
				Ok::<_, BevyError>((path, data))
			})
			.xmap(async_ext::try_join_all)
			.await
	}
}

impl BlobStoreProvider for BlobStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }
	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		self.provider.with_subdir(path)
	}
	fn id(&self) -> &'static str { self.provider.id() }
	fn root_key(&self) -> SmolStr { self.provider.root_key() }
	fn subdir(&self) -> SmolPath { self.provider.subdir() }
	fn watch_dir(&self) -> Option<AbsPathBuf> { self.provider.watch_dir() }
	fn base_dir(&self) -> Option<AbsPathBuf> { self.provider.base_dir() }
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.provider.did_change(event)
	}
	fn region(&self) -> Option<String> { self.provider.region() }
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.provider.store_exists()
	}
	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.provider.store_create()
	}
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		self.provider.store_remove()
	}
	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.provider.insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		self.provider.list()
	}
	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		self.provider.get(path)
	}
	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		self.provider.exists(path)
	}
	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		self.provider.remove(path)
	}
	fn public_url(
		&self,
		path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.provider.public_url(path)
	}
}

/// Test utilities for store providers.
#[cfg(test)]
pub mod store_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Runs the standard store provider test suite.
	pub async fn run(provider: impl BlobStoreProvider) {
		let store = BlobStore::new(provider);
		let path = SmolPath::from("test_path");
		let body = bytes::Bytes::from("test_body");
		store.store_remove().await.ok();
		store.store_exists().await.unwrap().xpect_false();
		store.store_try_create().await.unwrap();
		store.exists(&path).await.unwrap().xpect_false();
		store.remove(&path).await.xpect_err();
		store.insert(&path, body.clone()).await.unwrap();
		store.store_exists().await.unwrap().xpect_true();
		store.exists(&path).await.unwrap().xpect_true();
		store.list().await.unwrap().xpect_eq(vec![path.clone()]);
		store.get(&path).await.unwrap().xpect_eq(body.clone());
		store.get(&path).await.unwrap().xpect_eq(body);

		store.remove(&path).await.unwrap();
		store.get(&path).await.xpect_err();

		store.store_remove().await.unwrap();
		store.store_exists().await.unwrap().xpect_false();
	}
}
