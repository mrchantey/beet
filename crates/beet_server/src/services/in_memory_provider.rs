use crate::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bytes::Bytes;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;


impl Bucket {
	/// create a new [`FsBucketProvider`] in target/test_buckets
	pub async fn new_test() -> Self {
		let provider = InMemoryProvider::new();
		let bucket = Self::new(provider, "test-bucket");
		bucket.ensure_exists().await.unwrap();
		bucket
	}
}


/// A bucket provider using a simple nested hashmap
#[derive(Debug, Default, Clone)]
pub struct InMemoryProvider(
	pub Arc<RwLock<HashMap<String, HashMap<PathBuf, Bytes>>>>,
);

impl InMemoryProvider {
	pub fn new() -> Self { Self::default() }
}

impl BucketProvider for InMemoryProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
		let exists = self.0.read().unwrap().contains_key(bucket_name);
		Box::pin(async move { Ok(exists) })
	}

	fn create_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.0
			.write()
			.unwrap()
			.entry(bucket_name.to_string())
			.or_insert_with(HashMap::new);
		Box::pin(async { Ok(()) })
	}

	fn delete_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		self.0.write().unwrap().remove(bucket_name);
		Box::pin(async move { Ok(()) })
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let mut buckets = self.0.write().unwrap();
		buckets
			.entry(bucket_name.to_string())
			.or_insert_with(HashMap::new)
			.xmap(|bucket| {
				bucket.insert(path.to_path_buf(), body);
			});
		Box::pin(async move { Ok(()) })
	}

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>> {
		let mut buckets = self.0.write().unwrap();
		let result = buckets
			.get_mut(bucket_name)
			.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
			.map(|bucket| {
				bucket
					.get(&path.to_path_buf())
					.cloned()
					.ok_or_else(|| bevyhow!("item not found: {path}"))
			})
			.flatten();

		Box::pin(async move { result })
	}

	fn delete(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let mut buckets = self.0.write().unwrap();
		let result = buckets
			.get_mut(bucket_name)
			.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
			.map(|bucket| {
				bucket.remove(&path.to_path_buf());
			});

		Box::pin(async move { result })
	}

	fn public_url(
		&self,
		_bucket_name: &str,
		_path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'static>>
	{
		Box::pin(async move { Ok(None) })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	async fn works() {
		let provider = InMemoryProvider::new();
		super::super::bucket_test::run(provider).await;
	}
}
