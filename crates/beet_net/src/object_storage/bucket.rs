use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// Cross-service storage bucket for S3, filesystem, memory, or other providers
#[derive(Component)]
pub struct Bucket {
	/// Bucket name
	name: String,
	/// Provider that handles bucket operations (S3, filesystem, memory, etc.)
	provider: Box<dyn BucketProvider>,
}
impl Clone for Bucket {
	fn clone(&self) -> Self {
		Self {
			name: self.name.clone(),
			provider: self.provider.box_clone(),
		}
	}
}

impl Bucket {
	pub fn new(provider: impl BucketProvider, name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			provider: Box::new(provider),
		}
	}

	/// Create a [`BucketItem`] for working with a specific path
	///
	/// # Example
	/// ```no_run
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// let bucket = temp_bucket();
	/// let item = bucket.item(RoutePath::from("/my-file.txt"));
	/// ```
	pub fn item(&self, path: RoutePath) -> BucketItem {
		BucketItem::new(self.clone(), path)
	}

	/// Get bucket name
	pub fn name(&self) -> &str { &self.name }


	/// Create bucket (may take 10+ seconds for some services like dynamodb)
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
	/// Fails if bucket already exists
	pub async fn bucket_create(&self) -> Result {
		self.provider.bucket_create(&self.name).await
	}

	/// Ensure bucket exists, creating if needed
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
	pub async fn bucket_try_create(&self) -> Result {
		self.provider.bucket_try_create(&self.name).await
	}

	/// Check if bucket exists
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
	pub async fn bucket_exists(&self) -> Result<bool> {
		self.provider.bucket_exists(&self.name).await
	}
	/// Remove bucket
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
	/// Fails if bucket doesn't exist
	pub async fn bucket_remove(&self) -> Result {
		self.provider.bucket_remove(&self.name).await
	}
	/// Insert object into bucket
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.insert(&RoutePath::from("/file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(
		&self,
		path: &RoutePath,
		body: impl Into<Bytes>,
	) -> Result {
		self.provider.insert(&self.name, path, body.into()).await
	}

	/// Insert object, failing if it already exists
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.try_insert(&RoutePath::from("/file.txt"), "content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn try_insert(
		&self,
		path: &RoutePath,
		body: impl Into<Bytes>,
	) -> Result {
		if self.exists(path).await? {
			bevybail!("Object already exists: {}", path)
		} else {
			self.insert(path, body).await
		}
	}

	/// Check if object exists
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let exists = bucket.exists(&RoutePath::from("/file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self, path: &RoutePath) -> Result<bool> {
		self.provider.exists(&self.name, path).await
	}
	/// List all objects in bucket
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
	pub async fn list(&self) -> Result<Vec<RoutePath>> {
		self.provider.list(&self.name).await
	}
	/// Get object data
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let data = bucket.get(&RoutePath::from("/file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(&self, path: &RoutePath) -> Result<Bytes> {
		self.provider.get(&self.name, path).await
	}

	/// Get all objects and their data
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
	/// Expensive operation - prefer [`Self::list`] + [`Self::get`]
	pub async fn get_all(&self) -> Result<Vec<(RoutePath, Bytes)>> {
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

	/// Remove object from bucket
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// bucket.remove(&RoutePath::from("/file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove(&self, path: &RoutePath) -> Result {
		self.provider.remove(&self.name, path).await
	}

	/// Get public URL for object (if supported by provider)
	///
	/// # Example
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let bucket = temp_bucket();
	/// let url = bucket.public_url(&RoutePath::from("/file.txt")).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn public_url(&self, path: &RoutePath) -> Result<Option<String>> {
		self.provider.public_url(&self.name, path).await
	}
	/// Get provider region
	pub async fn region(&self) -> Option<String> { self.provider.region() }
}

pub trait BucketProvider: 'static + Send + Sync {
	fn box_clone(&self) -> Box<dyn BucketProvider>;

	/// Get provider region
	fn region(&self) -> Option<String>;
	/// Check if bucket exists
	fn bucket_exists(&self, bucket_name: &str)
	-> SendBoxedFuture<Result<bool>>;
	/// Create bucket
	fn bucket_create(&self, bucket_name: &str) -> SendBoxedFuture<Result>;
	/// Remove bucket (destructive operation!)
	fn bucket_remove(&self, bucket_name: &str) -> SendBoxedFuture<Result>;

	/// Ensure bucket exists, creating if needed
	fn bucket_try_create(&self, bucket_name: &str) -> SendBoxedFuture<Result> {
		let exists_fut = self.bucket_exists(bucket_name);
		let create_fut = self.bucket_create(bucket_name);
		Box::pin(async move {
			if exists_fut.await? {
				Ok(())
			} else {
				create_fut.await
			}
		})
	}
	/// Insert object into bucket
	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result>;
	/// List all objects in bucket
	fn list(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>>;
	/// Get object from bucket
	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>>;
	/// Check if object exists in bucket
	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>>;
	/// Remove object from bucket
	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result>;
	/// Get public URL of object
	/// - fs: `file:///data/buckets/my-bucket/key`
	/// - s3: `https://my-bucket.s3.us-west-2.amazonaws.com/key`
	fn public_url(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}

/// Create temporary in-memory bucket for testing
pub fn temp_bucket() -> Bucket { Bucket::new(InMemoryProvider::new(), "temp") }
/// Create local bucket with platform-specific provider
/// - wasm: [`LocalStorageProvider`]
/// - native: [`FsBucketProvider`] at `.cache/buckets`
pub fn local_bucket(name: impl Into<String>) -> Bucket {
	#[cfg(target_arch = "wasm32")]
	return Bucket::new(LocalStorageProvider::new(), name);
	#[cfg(not(target_arch = "wasm32"))]
	return Bucket::new(
		FsBucketProvider::new(
			AbsPathBuf::new_workspace_rel(".cache/buckets").unwrap(),
		),
		name,
	);
}
/// Select filesystem or S3 bucket based on [`ServiceAccess`] and feature flags
#[allow(unused_variables)]
pub async fn s3_fs_selector(
	fs_path: AbsPathBuf,
	bucket_name: impl AsRef<str>,
	access: ServiceAccess,
) -> Bucket {
	let bucket_name = bucket_name.as_ref();
	match access {
		ServiceAccess::Local => {
			debug!("Bucket Selector - FS: {fs_path}");
			Bucket::new(FsBucketProvider::new(fs_path.clone()), "")
		}
		#[cfg(not(all(feature = "aws", not(target_arch = "wasm32"))))]
		ServiceAccess::Remote => {
			debug!("Bucket Selector - FS (no aws or wasm): {fs_path}");
			Bucket::new(FsBucketProvider::new(fs_path.clone()), "")
		}
		#[cfg(all(feature = "aws", not(target_arch = "wasm32")))]
		ServiceAccess::Remote => {
			debug!("Bucket Selector - S3: {bucket_name}");
			let provider = S3Provider::create().await;
			Bucket::new(provider, bucket_name)
		}
	}
}



#[cfg(test)]
pub mod bucket_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	pub async fn run(provider: impl BucketProvider) {
		let bucket = Bucket::new(provider, "beet-test-bucket");
		let path = RoutePath::from("/test_path");
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
