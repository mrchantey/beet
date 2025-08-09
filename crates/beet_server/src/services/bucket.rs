use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use std::pin::Pin;

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
	/// Get the name of the bucket, ie `my-bucket`
	pub fn name(&self) -> &str { &self.name }

	/// Check if the bucket exists, creating it if necessary
	pub async fn ensure_exists(&self) -> Result<()> {
		self.provider.ensure_exists(&self.name).await
	}

	/// Check if the bucket exists
	pub async fn exists(&self) -> Result<bool> {
		self.provider.bucket_exists(&self.name).await
	}
	/// Remove the bucket
	pub async fn remove(&self) -> Result<()> {
		self.provider.delete_bucket(&self.name).await
	}

	/// Create the bucket if it does not exist
	pub async fn create(&self) -> Result<()> {
		self.provider.create_bucket(&self.name).await
	}

	pub async fn insert(
		&self,
		path: &RoutePath,
		body: impl Into<Bytes>,
	) -> Result<()> {
		self.provider.insert(&self.name, path, body.into()).await
	}
	pub async fn get(&self, path: &RoutePath) -> Result<Bytes> {
		self.provider.get(&self.name, path).await
	}
	pub async fn delete(&self, path: &RoutePath) -> Result<()> {
		self.provider.delete(&self.name, path).await
	}

	pub async fn public_url(&self, path: &RoutePath) -> Result<String> {
		self.provider.public_url(&self.name, path).await
	}
}


pub trait BucketProvider: 'static + Send + Sync {
	fn box_clone(&self) -> Box<dyn BucketProvider>;

	/// Get the region of the provider
	fn region(&self) -> Option<String>;
	/// Check if the bucket exists
	fn bucket_exists(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>>;
	/// Create the bucket
	fn create_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
	/// Delete the bucket
	fn delete_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

	/// Ensure the bucket exists, creating it if necessary
	fn ensure_exists(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let exists_fut = self.bucket_exists(bucket_name);
		let create_fut = self.create_bucket(bucket_name);
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
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
	/// Get an object from the bucket
	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>>;
	/// Delete an object from the bucket
	fn delete(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
	/// Get the public URL of an object in the bucket. For example:
	/// - fs `file:///data/buckets/my-bucket/key`
	/// - s3 `https://my-bucket.s3.us-west-2.amazonaws.com/key`
	fn public_url(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>>;
}
