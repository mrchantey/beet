use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// Trait for store storage backends (S3, filesystem, memory, etc.).
///
/// Implementations provide the actual storage operations for [`BlobStore`].
/// Each provider stores all required state internally (store name, region,
/// connection info, etc.) so that no external context is needed.
pub trait BlobStoreProvider: 'static + Send + Sync {
	/// Returns a boxed clone of this provider.
	fn box_clone(&self) -> Box<dyn BlobStoreProvider>;

	/// Returns a new provider scoped to the given subdirectory.
	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider>;

	/// Create a type-erased [`Blob`] handle for a single object managed by
	/// this provider. Prefer the typed [`FsStore::blob`], [`S3Store::blob`]
	/// etc. when you need world serialization.
	fn erased_blob(&self, path: SmolPath) -> Blob {
		Blob::new(BlobStore::new(self.box_clone()), path)
	}

	/// Stable family discriminator, ie `"fs"`, `"memory"`, `"localstorage"`,
	/// `"s3"`.
	fn id(&self) -> &'static str;

	/// [`id`](Self::id) plus the backing-instance identity, *without* the subdir.
	/// Two stores with the same `root_key` are the same effective store:
	/// - fs:           `"fs:{path}"`             (the base store directory)
	/// - memory:       `"memory:{instance_id}"`  (incrementing id per `new`)
	/// - localstorage: `"localstorage:{store_name}"`
	/// - s3:           `"s3:{bucket}"`           (unwatchable)
	fn root_key(&self) -> SmolStr;

	/// This store's `subdir` within its [`root_key`](Self::root_key), empty for
	/// the root. Used for per-event routing.
	fn subdir(&self) -> SmolPath { SmolPath::default() }

	/// The precise directory a native watcher should observe for this store, ie an
	/// [`FsStore`]'s `effective_root` (base joined with subdir). `None` for a store
	/// with no watchable directory (memory, S3). Drives [`WatchDir`](crate::prelude::WatchDir),
	/// so only the mounted subtree is watched, never the whole store root.
	#[cfg(feature = "std")]
	fn watch_dir(&self) -> Option<AbsPathBuf> { None }

	/// The store's base path, ie the directory its [`root_key`](Self::root_key) keys
	/// to, used to strip a watched path back to a base-relative [`BlobEvent`] so it
	/// routes via [`did_change`](Self::did_change). `None` for a non-fs store.
	#[cfg(feature = "std")]
	fn base_dir(&self) -> Option<AbsPathBuf> { None }

	/// True if `event` concerns an object inside this store's scope: same
	/// backing, and the event's root-relative location is this scope or a child
	/// of it.
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.root_key() == event.store.root_key()
			&& key_covers(&self.subdir(), &event.root_relative_path())
	}

	/// Returns the provider's region, if applicable.
	fn region(&self) -> Option<String>;

	/// Check if store exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let exists = store.store_exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>>;

	/// Create store (may take 10+ seconds for some services like DynamoDB).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.store_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if store already exists.
	fn store_create(&self) -> SendBoxedFuture<Result>;

	/// Remove store (destructive operation!).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.store_remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if store doesn't exist.
	fn store_remove(&self) -> SendBoxedFuture<Result>;

	/// Ensure store exists, creating if needed.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.store_try_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn store_try_create(&self) -> SendBoxedFuture<Result> {
		let exists_fut = self.store_exists();
		let create_fut = self.store_create();
		Box::pin(async move {
			if exists_fut.await? {
				Ok(())
			} else {
				create_fut.await
			}
		})
	}

	/// Check if store is empty (contains no objects).
	fn store_is_empty(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.box_clone();
		Box::pin(async move { this.list().await?.is_empty().xok() })
	}

	/// Insert object into store.
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
	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result>;

	/// List all objects in store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let paths = store.list().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>>;

	/// Get object from store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let data = store.get(&SmolPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>>;

	/// Check if object exists in store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let exists = store.exists(&SmolPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>>;

	/// Remove object from store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.remove(&SmolPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result>;

	/// Get public URL of object.
	/// - fs: `file:///data/stores/my-store/key`
	/// - s3: `https://my-store.s3.us-west-2.amazonaws.com/key`
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let url = store.public_url(&SmolPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn public_url(
		&self,
		path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}

impl BlobStoreProvider for Box<dyn BlobStoreProvider> {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> {
		self.as_ref().box_clone()
	}
	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		self.as_ref().with_subdir(path)
	}
	fn id(&self) -> &'static str { self.as_ref().id() }
	fn root_key(&self) -> SmolStr { self.as_ref().root_key() }
	fn subdir(&self) -> SmolPath { self.as_ref().subdir() }
	#[cfg(feature = "std")]
	fn watch_dir(&self) -> Option<AbsPathBuf> { self.as_ref().watch_dir() }
	#[cfg(feature = "std")]
	fn base_dir(&self) -> Option<AbsPathBuf> { self.as_ref().base_dir() }
	fn did_change(&self, event: &BlobEvent) -> bool {
		self.as_ref().did_change(event)
	}
	fn region(&self) -> Option<String> { self.as_ref().region() }
	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.as_ref().store_exists()
	}
	fn store_create(&self) -> SendBoxedFuture<Result> {
		self.as_ref().store_create()
	}
	fn store_remove(&self) -> SendBoxedFuture<Result> {
		self.as_ref().store_remove()
	}
	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.as_ref().insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		self.as_ref().list()
	}
	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		self.as_ref().get(path)
	}
	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		self.as_ref().exists(path)
	}
	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		self.as_ref().remove(path)
	}
	fn public_url(
		&self,
		path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.as_ref().public_url(path)
	}
}
