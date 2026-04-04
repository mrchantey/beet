use crate::prelude::*;
use async_lock::RwLock;
use beet_core::prelude::*;
use bytes::Bytes;
use std::sync::Arc;


impl Bucket {
	/// Create a pre-created [`InMemoryProvider`] bucket for testing.
	pub fn new_test() -> Self { Self::new(InMemoryProvider::created()) }
}


/// A bucket provider using an in-memory hashmap.
///
/// Inner state is `None` when the bucket has not been created,
/// and `Some(map)` when it exists.
#[derive(Debug, Default, Clone, Component)]
pub struct InMemoryProvider(pub Arc<RwLock<Option<HashMap<RoutePath, Bytes>>>>);

impl InMemoryProvider {
	/// Creates a new uncreated in-memory provider.
	pub fn new() -> Self { Self::default() }

	/// Creates a new already-created (empty) in-memory provider.
	pub fn created() -> Self {
		Self(Arc::new(RwLock::new(Some(HashMap::new()))))
	}
}

#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for InMemoryProvider {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BucketProvider for InMemoryProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move { this.0.read().await.is_some().xok() })
	}

	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.0.write().await;
			if guard.is_some() {
				bevybail!("bucket already exists")
			}
			*guard = Some(HashMap::new());
			Ok(())
		})
	}

	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.0.write().await;
			if guard.is_none() {
				bevybail!("bucket does not exist")
			}
			*guard = None;
			Ok(())
		})
	}

	fn insert(&self, path: &RoutePath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let path = path.clone();
		Box::pin(async move {
			let mut guard = this.0.write().await;
			let map = guard
				.as_mut()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.insert(path, body);
			Ok(())
		})
	}

	fn exists(&self, path: &RoutePath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let path = path.clone();
		Box::pin(async move {
			let guard = this.0.read().await;
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.contains_key(&path).xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let this = self.clone();
		Box::pin(async move {
			let guard = this.0.read().await;
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.keys().cloned().collect::<Vec<_>>().xok()
		})
	}

	fn get(&self, path: &RoutePath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let path = path.clone();
		Box::pin(async move {
			let guard = this.0.read().await;
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.get(&path)
				.cloned()
				.ok_or_else(|| bevyhow!("object not found: {path}"))
		})
	}

	fn remove(&self, path: &RoutePath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let path = path.clone();
		Box::pin(async move {
			let mut guard = this.0.write().await;
			let map = guard
				.as_mut()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.remove(&path)
				.map(|_| ())
				.ok_or_else(|| bevyhow!("object not found: {path}"))
		})
	}

	fn public_url(
		&self,
		_path: &RoutePath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { None.xok() })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[beet_core::test]
	async fn works() {
		let provider = InMemoryProvider::new();
		bucket_test::run(provider).await;
	}
}
