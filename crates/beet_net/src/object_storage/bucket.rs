use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use std::sync::Arc;

/// Cross-service storage bucket for S3, filesystem, memory, or other providers.
///
/// Wraps an [`Arc<dyn BucketProvider>`] and delegates all operations to the
/// inner provider. Implements [`BucketProvider`] itself, so it can be used
/// anywhere a provider is expected.
#[derive(Clone, Component)]
pub struct Bucket {
	/// Provider that handles bucket operations (S3, filesystem, memory, etc.)
	provider: Arc<dyn BucketProvider>,
}

impl std::fmt::Debug for Bucket {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Bucket").finish_non_exhaustive()
	}
}

impl Bucket {
	/// Creates a new bucket wrapping the given provider.
	pub fn new(provider: impl BucketProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// Creates a new bucket from a pre-existing [`Arc<dyn BucketProvider>`].
	pub fn from_arc(provider: Arc<dyn BucketProvider>) -> Self {
		Self { provider }
	}

	/// Create a [`BucketItem`] for working with a specific path.
	///
	/// # Example
	/// ```no_run
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let bucket = temp_bucket();
	/// let item = bucket.item(RelPath::from("my-file.txt"));
	/// ```
	pub fn item(&self, path: RelPath) -> BucketItem {
		BucketItem::new(self.clone(), path)
	}

	/// Create a [`Blob`] handle for a single object in this bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let bucket = temp_bucket();
	/// let blob = bucket.blob(RelPath::new("my-file.txt"));
	/// ```
	pub fn blob(&self, path: RelPath) -> Blob {
		Blob::new(self.provider.box_clone(), path)
	}

	/// Insert object into bucket.
	///
	/// Ergonomic wrapper accepting any `impl Into<Bytes>` (eg. `&str`, `Vec<u8>`).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(
		&self,
		path: &RelPath,
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
	/// let bucket = temp_bucket();
	/// bucket.try_insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn try_insert(
		&self,
		path: &RelPath,
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
	/// let bucket = temp_bucket();
	/// let all_data = bucket.get_all().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Caution
	/// Expensive operation - prefer [`BucketProvider::list`] + [`BucketProvider::get`]
	pub async fn get_all(&self) -> Result<Vec<(RelPath, Bytes)>> {
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

impl BucketProvider for Bucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }
	fn region(&self) -> Option<String> { self.provider.region() }
	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.provider.bucket_exists()
	}
	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		self.provider.bucket_create()
	}
	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		self.provider.bucket_remove()
	}
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		self.provider.insert(path, body)
	}
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		self.provider.list()
	}
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		self.provider.get(path)
	}
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		self.provider.exists(path)
	}
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		self.provider.remove(path)
	}
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		self.provider.public_url(path)
	}
}

/// Trait for bucket storage backends (S3, filesystem, memory, etc.).
///
/// Implementations provide the actual storage operations for [`Bucket`].
/// Each provider stores all required state internally (bucket name, region,
/// connection info, etc.) so that no external context is needed.
pub trait BucketProvider: 'static + Send + Sync {
	/// Returns a boxed clone of this provider.
	fn box_clone(&self) -> Box<dyn BucketProvider>;

	/// Create a [`Blob`] handle for a single object managed by this provider.
	fn blob(&self, path: RelPath) -> Blob { Blob::new(self.box_clone(), path) }

	/// Returns the provider's region, if applicable.
	fn region(&self) -> Option<String>;

	/// Check if bucket exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let exists = bucket.bucket_exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>>;

	/// Create bucket (may take 10+ seconds for some services like DynamoDB).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.bucket_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if bucket already exists.
	fn bucket_create(&self) -> SendBoxedFuture<Result>;

	/// Remove bucket (destructive operation!).
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.bucket_remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	///
	/// # Errors
	/// Fails if bucket doesn't exist.
	fn bucket_remove(&self) -> SendBoxedFuture<Result>;

	/// Ensure bucket exists, creating if needed.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.bucket_try_create().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn bucket_try_create(&self) -> SendBoxedFuture<Result> {
		let exists_fut = self.bucket_exists();
		let create_fut = self.bucket_create();
		Box::pin(async move {
			if exists_fut.await? {
				Ok(())
			} else {
				create_fut.await
			}
		})
	}

	/// Check if bucket is empty (contains no objects).
	fn bucket_is_empty(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.box_clone();
		Box::pin(async move { this.list().await?.is_empty().xok() })
	}

	/// Insert object into bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.insert(&RelPath::from("file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result>;

	/// List all objects in bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let paths = bucket.list().await?;
	/// # Ok(())
	/// # }
	/// ```
	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>>;

	/// Get object from bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let data = bucket.get(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>>;

	/// Check if object exists in bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let exists = bucket.exists(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>>;

	/// Remove object from bucket.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.remove(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result>;

	/// Get public URL of object.
	/// - fs: `file:///data/buckets/my-bucket/key`
	/// - s3: `https://my-bucket.s3.us-west-2.amazonaws.com/key`
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let url = bucket.public_url(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}

/// Create temporary in-memory bucket for testing.
/// The returned bucket is pre-created and ready for immediate use.
pub fn temp_bucket() -> Bucket { Bucket::new(InMemoryProvider::created()) }

/// Create local bucket with platform-specific provider.
/// - wasm: [`LocalStorageProvider`]
/// - native: [`FsBucketProvider`] at `.cache/buckets/<name>`
pub fn local_bucket(name: impl Into<String>) -> Bucket {
	let name = name.into();
	#[cfg(target_arch = "wasm32")]
	return Bucket::new(LocalStorageProvider::new(name));
	#[cfg(not(target_arch = "wasm32"))]
	return Bucket::new(FsBucketProvider::new(
		AbsPathBuf::new_workspace_rel(format!(".cache/buckets/{name}"))
			.unwrap(),
	));
}

/// Select filesystem or S3 bucket based on [`ServiceAccess`] and feature flags.
#[allow(unused_variables)]
pub async fn s3_fs_selector(
	fs_path: AbsPathBuf,
	bucket_name: impl AsRef<str>,
	region: &str,
	access: ServiceAccess,
) -> Bucket {
	let bucket_name = bucket_name.as_ref();
	match access {
		ServiceAccess::Local => {
			debug!("Bucket Selector - FS: {fs_path}");
			Bucket::new(FsBucketProvider::new(fs_path))
		}
		#[cfg(not(all(feature = "aws", not(target_arch = "wasm32"))))]
		ServiceAccess::Remote => {
			debug!("Bucket Selector - FS (no aws or wasm): {fs_path}");
			Bucket::new(FsBucketProvider::new(fs_path))
		}
		#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
		ServiceAccess::Remote => {
			debug!("Bucket Selector - S3: {bucket_name}");
			let provider = S3Provider::new(bucket_name, region);
			Bucket::new(provider)
		}
	}
}

/// Test utilities for bucket providers.
#[cfg(test)]
pub mod bucket_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Runs the standard bucket provider test suite.
	pub async fn run(provider: impl BucketProvider) {
		let bucket = Bucket::new(provider);
		let path = RelPath::from("test_path");
		let body = bytes::Bytes::from("test_body");
		bucket.bucket_remove().await.ok();
		bucket.bucket_exists().await.unwrap().xpect_false();
		bucket.bucket_try_create().await.unwrap();
		bucket.exists(&path).await.unwrap().xpect_false();
		bucket.remove(&path).await.xpect_err();
		bucket.insert(&path, body.clone()).await.unwrap();
		bucket.bucket_exists().await.unwrap().xpect_true();
		bucket.exists(&path).await.unwrap().xpect_true();
		bucket.list().await.unwrap().xpect_eq(vec![path.clone()]);
		bucket.get(&path).await.unwrap().xpect_eq(body.clone());
		bucket.get(&path).await.unwrap().xpect_eq(body);

		bucket.remove(&path).await.unwrap();
		bucket.get(&path).await.xpect_err();

		bucket.bucket_remove().await.unwrap();
		bucket.bucket_exists().await.unwrap().xpect_false();
	}
}
