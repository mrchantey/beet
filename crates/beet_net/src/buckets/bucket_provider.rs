use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// Trait for bucket storage backends (S3, filesystem, memory, etc.).
///
/// Implementations provide the actual storage operations for [`Bucket`].
/// Each provider stores all required state internally (bucket name, region,
/// connection info, etc.) so that no external context is needed.
pub trait BucketProvider: 'static + Send + Sync {
	/// Returns a boxed clone of this provider.
	fn box_clone(&self) -> Box<dyn BucketProvider>;

	/// Returns a new provider scoped to the given subdirectory.
	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider>;

	/// Create a type-erased [`Blob`] handle for a single object managed by
	/// this provider. Prefer the typed [`FsBucket::blob`], [`S3Bucket::blob`]
	/// etc. when you need scene serialization.
	fn erased_blob(&self, path: RelPath) -> Blob {
		Blob::new(Bucket::new(self.box_clone()), path)
	}

	/// Returns the provider's region, if applicable.
	fn region(&self) -> Option<String>;

	/// Check if bucket exists.
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
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
	/// let bucket = Bucket::temp();
	/// let url = bucket.public_url(&RelPath::from("file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	fn public_url(
		&self,
		path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}

impl BucketProvider for Box<dyn BucketProvider> {
	fn box_clone(&self) -> Box<dyn BucketProvider> { self.as_ref().box_clone() }
	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		self.as_ref().with_subdir(path)
	}
	fn region(&self) -> Option<String> { self.as_ref().region() }
	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		self.as_ref().bucket_exists()
	}
	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		self.as_ref().bucket_create()
	}
	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		self.as_ref().bucket_remove()
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
