use beet_core::prelude::*;

/// What every store backend is, whichever kind of thing it holds: an identity
/// and a lifecycle. Bytes are a [`BlobProvider`](crate::prelude::BlobProvider)'s and rows a
/// `TableProvider`'s, two traits over this one, neither implying the other.
pub trait StoreBackend: 'static + Send + Sync {
	/// Stable family discriminator, ie `"fs"`, `"memory"`, `"localstorage"`,
	/// `"s3"`, `"sqlite"`.
	fn id(&self) -> &'static str;

	/// Where this store actually lives, ie `s3:beet-site--prod--app
	/// (us-west-2)`. The one thing an operator needs from a store that will not
	/// answer, so it belongs in any error naming one.
	fn describe(&self) -> String;

	/// Returns the backend's region, if applicable.
	fn region(&self) -> Option<String>;

	/// Check if store exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::new(InMemoryBlobStore::new());
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
	/// let store = BlobStore::new(InMemoryBlobStore::new());
	/// store.store_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if store already exists.
	fn store_create(&self) -> SendBoxedFuture<Result>;

	/// Ensure store exists, creating if needed.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::new(InMemoryBlobStore::new());
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

	/// Remove store (destructive operation!).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let store = BlobStore::new(InMemoryBlobStore::new());
	/// store.store_remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if store doesn't exist.
	fn store_remove(&self) -> SendBoxedFuture<Result>;
}
