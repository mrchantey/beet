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
	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider>;

	/// Create a type-erased [`Blob`] handle for a single object managed by
	/// this provider. Prefer the typed [`FsStore::blob`], [`S3Store::blob`]
	/// etc. when you need scene serialization.
	fn erased_blob(&self, path: RelPath) -> Blob {
		Blob::new(BlobStore::new(self.box_clone()), path)
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
	/// store.insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result>;

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
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>>;

	/// Get object from store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let data = store.get(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>>;

	/// Check if object exists in store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// let exists = store.exists(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>>;

	/// Remove object from store.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::temp();
	/// store.remove(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result>;

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
	/// let url = store.public_url(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}

impl BlobStoreProvider for Box<dyn BlobStoreProvider> {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { self.as_ref().box_clone() }
	fn with_subdir(&self, path: RelPath) -> Box<dyn BlobStoreProvider> {
		self.as_ref().with_subdir(path)
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
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.as_ref().insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		self.as_ref().list()
	}
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		self.as_ref().get(path)
	}
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		self.as_ref().exists(path)
	}
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		self.as_ref().remove(path)
	}
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.as_ref().public_url(path)
	}
}
