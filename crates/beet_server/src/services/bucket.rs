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
		key: &str,
		body: impl Into<Bytes>,
	) -> Result<()> {
		self.provider.insert(&self.name, key, body.into()).await
	}
	pub async fn get(&self, key: &str) -> Result<Bytes> {
		self.provider.get(&self.name, key).await
	}
	pub async fn delete(&self, key: &str) -> Result<()> {
		self.provider.delete(&self.name, key).await
	}

	pub async fn public_url(&self, key: &str) -> Result<String> {
		self.provider.public_url(&self.name, key).await
	}
}


pub trait BucketProvider: 'static + Send + Sync {
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
		key: &str,
		body: Bytes,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
	/// Get an object from the bucket
	fn get(
		&self,
		bucket_name: &str,
		key: &str,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>>;
	/// Delete an object from the bucket
	fn delete(
		&self,
		bucket_name: &str,
		key: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
	/// Get the public URL of an object in the bucket. For example:
	/// - fs `file:///data/buckets/my-bucket/key`
	/// - s3 `https://my-bucket.s3.us-west-2.amazonaws.com/key`
	fn public_url(
		&self,
		bucket_name: &str,
		key: &str,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>>;
}
