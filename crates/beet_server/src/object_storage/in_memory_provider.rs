use crate::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;


impl Bucket {
	/// create a new [`FsBucketProvider`] in target/test_buckets
	pub async fn new_test() -> Self {
		let provider = InMemoryProvider::new();
		let bucket = Self::new(provider, "test-bucket");
		bucket.bucket_try_create().await.unwrap();
		bucket
	}
}


/// A bucket provider using a simple nested hashmap
#[derive(Debug, Default, Clone)]
pub struct InMemoryProvider(
	pub Arc<RwLock<HashMap<String, HashMap<RoutePath, Bytes>>>>,
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

	fn bucket_create(
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

	fn bucket_remove(
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
				bucket.insert(path.clone(), body);
			});
		Box::pin(async move { Ok(()) })
	}

	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
		let mut buckets = self.0.write().unwrap();
		let result = buckets
			.get_mut(bucket_name)
			.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
			.map(|bucket| bucket.contains_key(path));

		Box::pin(async move { result })
	}

	fn list(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<Vec<RoutePath>>> + Send + 'static>>
	{
		let mut buckets = self.0.write().unwrap();
		let result = buckets
			.get_mut(bucket_name)
			.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
			.map(|bucket| bucket.keys().cloned().collect());

		Box::pin(async move { result })
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
					.get(path)
					.cloned()
					.ok_or_else(|| bevyhow!("Object not found: {path}"))
			})
			.flatten();

		Box::pin(async move { result })
	}

	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let mut buckets = self.0.write().unwrap();
		let result = buckets
			.get_mut(bucket_name)
			.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
			.and_then(|bucket| {
				bucket
					.remove(path)
					.map(|_| ())
					.ok_or_else(|| bevyhow!("Object not found: {path}"))
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
		bucket_test::run(provider).await;
	}
}
