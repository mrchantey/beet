use crate::prelude::*;
use async_lock::RwLock;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bytes::Bytes;
use std::sync::Arc;


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
#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct InMemoryProvider(
	pub Arc<RwLock<HashMap<String, HashMap<RoutePath, Bytes>>>>,
);

impl InMemoryProvider {
	pub fn new() -> Self { Self::default() }
}

impl<T: TableRow> TableProvider<T> for InMemoryProvider {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BucketProvider for InMemoryProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(
			async move { this.read().await.contains_key(&bucket_name).xok() },
		)
	}

	fn bucket_create(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			this.write()
				.await
				.entry(bucket_name)
				.or_insert_with(HashMap::new);
			Ok(())
		})
	}

	fn bucket_remove(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			this.write().await.remove(&bucket_name);
			Ok(())
		})
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result<()>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		let path = path.clone();
		Box::pin(async move {
			this.write()
				.await
				.entry(bucket_name)
				.or_insert_with(HashMap::new)
				.insert(path, body);
			Ok(())
		})
	}

	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		let path = path.clone();
		Box::pin(async move {
			this.write()
				.await
				.get(&bucket_name)
				.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
				.map(|bucket| bucket.contains_key(&path))
		})
	}

	fn list(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		Box::pin(async move {
			this.write()
				.await
				.get(&bucket_name)
				.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
				.map(|bucket| bucket.keys().cloned().collect())
		})
	}

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		let path = path.clone();
		Box::pin(async move {
			this.write()
				.await
				.get(&bucket_name)
				.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
				.map(|bucket| {
					bucket
						.get(&path)
						.cloned()
						.ok_or_else(|| bevyhow!("Object not found: {path}"))
				})
				.flatten()
		})
	}

	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<()>> {
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		let path = path.clone();
		Box::pin(async move {
			this.write()
				.await
				.get_mut(&bucket_name)
				.ok_or_else(|| bevyhow!("bucket not found: {bucket_name}"))
				.and_then(|bucket| {
					bucket
						.remove(&path)
						.map(|_| ())
						.ok_or_else(|| bevyhow!("Object not found: {path}"))
				})
		})
	}

	fn public_url(
		&self,
		_bucket_name: &str,
		_path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>> {
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
