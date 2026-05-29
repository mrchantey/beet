use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::RwLock;
use bytes::Bytes;


impl BlobStore {
	/// Create a pre-created [`InMemoryStore`] store for testing.
	pub fn new_test() -> Self { Self::new(InMemoryStore::new()) }
}


/// A store provider using an in-memory hashmap.
///
/// Inner state is `None` when the store has not been created,
/// and `Some(map)` when it exists. An optional `subdir` scopes
/// all operations to a key prefix.
#[derive(Debug, Clone)]
pub struct InMemoryStore {
	/// Shared storage state.
	inner: Arc<RwLock<Option<HashMap<SmolPath, Bytes>>>>,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<SmolPath>,
}

impl Default for InMemoryStore {
	fn default() -> Self { Self::new() }
}

impl InMemoryStore {
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
	fn resolve_key(&self, path: &SmolPath) -> SmolPath {
		match &self.subdir {
			Some(sub) => sub.join(path),
			None => path.clone(),
		}
	}
}

#[cfg(all(feature = "json", feature = "std"))]
impl<T: TableStoreRow> TableProvider<T> for InMemoryStore {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BlobStoreProvider for InMemoryStore {
	fn box_clone(&self) -> Box<dyn BlobStoreProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: SmolPath) -> Box<dyn BlobStoreProvider> {
		Box::new(InMemoryStore {
			inner: self.inner.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn region(&self) -> Option<String> { None }

	fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		Box::pin(async move { this.inner.read().unwrap().is_some().xok() })
	}

	fn store_create(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.inner.write().unwrap();
			if guard.is_some() {
				bevybail!("store already exists")
			}
			*guard = Some(HashMap::new());
			Ok(())
		})
	}

	fn store_remove(&self) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move {
			let mut guard = this.inner.write().unwrap();
			if guard.is_none() {
				bevybail!("store does not exist")
			}
			*guard = None;
			Ok(())
		})
	}

	fn insert(&self, path: &SmolPath, body: Bytes) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let mut guard = this.inner.write().unwrap();
			let map = guard
				.as_mut()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.insert(key, body);
			Ok(())
		})
	}

	fn exists(&self, path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.contains_key(&key).xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
		let this = self.clone();
		Box::pin(async move {
			let guard = this.inner.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			match &this.subdir {
				Some(sub) => {
					// filter keys by prefix and strip it
					map.keys()
						.filter_map(|key| {
							key.as_str()
								.strip_prefix(sub.as_str())
								.map(SmolPath::new)
						})
						.collect::<Vec<_>>()
						.xok()
				}
				None => map.keys().cloned().collect::<Vec<_>>().xok(),
			}
		})
	}

	fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let guard = this.inner.read().unwrap();
			let map = guard
				.as_ref()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.get(&key)
				.cloned()
				.ok_or_else(|| bevyhow!("object not found: {key}"))
		})
	}

	fn remove(&self, path: &SmolPath) -> SendBoxedFuture<Result> {
		let this = self.clone();
		let key = self.resolve_key(path);
		Box::pin(async move {
			let mut guard = this.inner.write().unwrap();
			let map = guard
				.as_mut()
				.ok_or_else(|| bevyhow!("store not created"))?;
			map.remove(&key)
				.map(|_| ())
				.ok_or_else(|| bevyhow!("object not found: {key}"))
		})
	}

	fn public_url(
		&self,
		_path: &SmolPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { None.xok() })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[beet_core::test]
	async fn works() {
		let provider = InMemoryStore::new_empty();
		store_test::run(provider).await;
	}
}
