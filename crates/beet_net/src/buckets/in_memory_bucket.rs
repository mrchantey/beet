use crate::prelude::*;
use async_lock::RwLock;
use beet_core::prelude::*;
use bytes::Bytes;
use std::sync::Arc;


impl Bucket {
	/// Create a pre-created [`InMemoryBucket`] bucket for testing.
	pub fn new_test() -> Self { Self::new(InMemoryBucket::new()) }
}


/// A bucket provider using an in-memory hashmap.
///
/// Inner state is `None` when the bucket has not been created,
/// and `Some(map)` when it exists. An optional `subdir` scopes
/// all operations to a key prefix.
#[derive(Debug, Clone)]
pub struct InMemoryBucket {
	/// Shared storage state.
	inner: Arc<RwLock<Option<HashMap<RelPath, Bytes>>>>,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<RelPath>,
}

impl Default for InMemoryBucket {
	fn default() -> Self { Self::new() }
}

impl InMemoryBucket {
	/// Creates a new already-created (empty) in-memory provider.
	pub fn new() -> Self {
		Self {
			inner: Arc::new(RwLock::new(Some(HashMap::new()))),
			subdir: None,
		}
	}
	/// Creates a new uncreated in-memory provider.
	pub fn new_empty() -> Self {
		Self {
			inner: Arc::new(RwLock::new(None)),
			subdir: None,
		}
	}

	/// Resolve an external path to the internal key by prepending the subdir.
	fn resolve_key(&self, path: &RelPath) -> RelPath {
		match &self.subdir {
			Some(sub) => sub.join(path),
			None => path.clone(),
		}
	}
}

#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for InMemoryBucket {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BucketProvider for InMemoryBucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		Box::new(InMemoryBucket {
			inner: self.inner.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move { this.inner.read().await.is_some().xok() })
	}

	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.inner.write().await;
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
			let mut guard = this.inner.write().await;
			if guard.is_none() {
				bevybail!("bucket does not exist")
			}
			*guard = None;
			Ok(())
		})
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let mut guard = this.inner.write().await;
			let map = guard
				.as_mut()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.insert(key, body);
			Ok(())
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.read().await;
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.contains_key(&key).xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let this = self.clone();
		Box::pin(async move {
			let guard = this.inner.read().await;
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			match &this.subdir {
				Some(sub) => {
					// filter keys by prefix and strip it
					map.keys()
						.filter_map(|key| {
							key.strip_prefix(sub.as_path())
								.ok()
								.map(|stripped| RelPath::new(stripped))
						})
						.collect::<Vec<_>>()
						.xok()
				}
				None => map.keys().cloned().collect::<Vec<_>>().xok(),
			}
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.read().await;
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.get(&key)
				.cloned()
				.ok_or_else(|| bevyhow!("object not found: {key}"))
		})
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let mut guard = this.inner.write().await;
			let map = guard
				.as_mut()
				.ok_or_else(|| bevyhow!("bucket not created"))?;
			map.remove(&key)
				.map(|_| ())
				.ok_or_else(|| bevyhow!("object not found: {key}"))
		})
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { None.xok() })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[beet_core::test]
	async fn works() {
		let provider = InMemoryBucket::new_empty();
		bucket_test::run(provider).await;
	}
}
