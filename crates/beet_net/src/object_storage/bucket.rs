use crate::prelude::*;
use beet_core::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;

/// Cross-service storage bucket representation
#[derive(Component)]
pub struct Bucket {
	/// The resource name of the bucket.
	name: String,
	/// The provider that handles the bucket operations.
	/// This may be S3, a local filesystem, or any other storage provider.
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
	/// Create a new bucket with a platform dependent provider
	/// - wasm: [`LocalStorageProvider`]
	/// - native: [`FsBucketProvider`] with root: `.cache/buckets`
	pub fn new_local(name: impl Into<String>) -> Self {
		#[cfg(target_arch = "wasm32")]
		return Self::new(LocalStorageProvider::new(), name);
		#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
		return Self::new(
			FsBucketProvider::new(
				AbsPathBuf::new_workspace_rel(".cache/buckets").unwrap(),
			),
			name,
		);
		#[cfg(not(any(target_arch = "wasm32", feature = "tokio")))]
		panic!(
			"Failed to create bucket {}, Bucket::new_local requires either the wasm32 target or the \"tokio\" feature to be enabled.",
			name.into()
		);
	}

	/// Create a [`BucketItem`] for interactively working with a bucket item
	pub fn item(&self, path: RoutePath) -> BucketItem {
		BucketItem::new(self.clone(), path)
	}

	/// Get the name of the bucket, ie `my-bucket`
	pub fn name(&self) -> &str { &self.name }


	/// Create the bucket if it does not exist
	pub async fn bucket_create(&self) -> Result<()> {
		self.provider.bucket_create(&self.name).await
	}

	/// Check if the bucket exists, creating it if necessary
	pub async fn bucket_try_create(&self) -> Result<()> {
		self.provider.bucket_try_create(&self.name).await
	}

	/// Check if the bucket exists
	pub async fn bucket_exists(&self) -> Result<bool> {
		self.provider.bucket_exists(&self.name).await
	}
	/// Remove the bucket
	pub async fn bucket_remove(&self) -> Result<()> {
		self.provider.bucket_remove(&self.name).await
	}
	/// Insert an object into the bucket
	pub async fn insert(
		&self,
		path: &RoutePath,
		body: impl Into<Bytes>,
	) -> Result<()> {
		self.provider.insert(&self.name, path, body.into()).await
	}

	/// Try to insert the object, returning an error if it already exists
	pub async fn try_insert(
		&self,
		path: &RoutePath,
		body: impl Into<Bytes>,
	) -> Result<()> {
		if self.exists(path).await? {
			bevybail!("Object already exists: {}", path)
		} else {
			self.insert(path, body).await
		}
	}

	/// Check if the object exists
	pub async fn exists(&self, path: &RoutePath) -> Result<bool> {
		self.provider.exists(&self.name, path).await
	}
	/// List all objects in this bucket
	pub async fn list(&self) -> Result<Vec<RoutePath>> {
		self.provider.list(&self.name).await
	}
	/// Get the data for the object at the specified path
	pub async fn get(&self, path: &RoutePath) -> Result<Bytes> {
		self.provider.get(&self.name, path).await
	}

	/// Get the data for all items in this bucket
	///
	/// ## Caution
	/// This is potentially a very expensive operation and should
	/// only be used in special circumstances, prefer [`Self::list`]
	/// and [`Self::get`] in most circumstances.
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

	/// Remove an object from the bucket
	pub async fn remove(&self, path: &RoutePath) -> Result<()> {
		self.provider.remove(&self.name, path).await
	}

	/// Get the public URL for the object at the specified path, if applicable
	pub async fn public_url(&self, path: &RoutePath) -> Result<Option<String>> {
		self.provider.public_url(&self.name, path).await
	}
	/// Get the region of this buckets [`BucketProvider`]
	pub async fn region(&self) -> Option<String> { self.provider.region() }
}

pub trait BucketProvider: 'static + Send + Sync {
	fn box_clone(&self) -> Box<dyn BucketProvider>;

	/// Get the region of the provider
	fn region(&self) -> Option<String>;
	/// Check if the bucket exists
	fn bucket_exists(&self, bucket_name: &str)
	-> SendBoxedFuture<Result<bool>>;
	/// Create the bucket
	fn bucket_create(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>>;
	/// Remove the bucket
	/// ## Caution
	/// This operation is potentially very destructive, use with care!
	fn bucket_remove(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>>;

	/// Ensure the bucket exists, creating it if necessary
	fn bucket_try_create(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<()>> {
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
	/// Insert an object into the bucket
	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result<()>>;
	/// List all items in the bucket
	fn list(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>>;
	/// Get an object from the bucket
	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>>;
	/// Check if an object exists in the bucket
	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>>;
	/// Delete an object from the bucket
	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<()>>;
	/// Get the public URL of an object in the bucket. For example:
	/// - fs `file:///data/buckets/my-bucket/key`
	/// - s3 `https://my-bucket.s3.us-west-2.amazonaws.com/key`
	fn public_url(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>>;
}




#[cfg(test)]
pub mod bucket_test {
	use crate::prelude::*;
	use sweet::prelude::*;

	pub async fn run(provider: impl BucketProvider) {
		let bucket = Bucket::new(provider, "beet-test-bucket-849302");
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
