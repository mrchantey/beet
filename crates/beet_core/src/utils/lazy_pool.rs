use crate::prelude::*;
use async_lock::RwLock;

/// A simple pool that lazily initializes values on demand,
/// with support for both verbatim and [`Result`] constructors.
///
/// Uses double-checked locking to avoid TOCTOU races where
/// concurrent callers both miss the read cache and both construct.
pub struct LazyPool<K, V, O> {
	map: RwLock<HashMap<K, V>>,
	constructor: fn(&K) -> BoxedFuture<O>,
}

impl<K, V, O> LazyPool<K, V, O> {
	/// Creates a new `LazyPool` with the given constructor function.
	pub const fn new(constructor: fn(&K) -> BoxedFuture<O>) -> Self {
		Self {
			map: RwLock::new(HashMap::new()),
			constructor,
		}
	}
}


impl<K, V> LazyPool<K, V, V>
where
	K: std::hash::Hash + Eq + Clone,
	V: Clone,
{
	/// Gets the value for the given key, initializing it if it doesn't exist.
	///
	/// Uses double-checked locking: first checks under a read lock, then
	/// re-checks under a write lock before constructing to prevent duplicate
	/// initialization from concurrent callers.
	pub async fn get(&self, key: &K) -> V {
		if let Some(value) = self.map.read().await.get(key) {
			return value.clone();
		}
		let mut map = self.map.write().await;
		if let Some(value) = map.get(key) {
			return value.clone();
		}
		let value = (self.constructor)(key).await;
		map.insert(key.clone(), value.clone());
		value
	}
}

impl<K, V> LazyPool<K, V, Result<V>>
where
	K: std::hash::Hash + Eq + Clone,
	V: Clone,
{
	/// Gets the value for the given key, initializing it if it doesn't exist.
	///
	/// Uses double-checked locking: first checks under a read lock, then
	/// re-checks under a write lock before constructing to prevent duplicate
	/// initialization from concurrent callers.
	pub async fn try_get(&self, key: &K) -> Result<V> {
		if let Some(value) = self.map.read().await.get(key) {
			return value.clone().xok();
		}
		let mut map = self.map.write().await;
		if let Some(value) = map.get(key) {
			return value.clone().xok();
		}
		let value = (self.constructor)(key).await?;
		map.insert(key.clone(), value.clone());
		value.xok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[crate::test]
	async fn works() {
		static POOL: LazyPool<u32, String, String> = LazyPool::new(|key| {
			Box::pin(async move { format!("Value for key {}", key) })
		});
		let value = POOL.get(&42).await;
		assert_eq!(value, "Value for key 42");
	}

	#[crate::test]
	async fn deduplicates() {
		use std::sync::atomic::AtomicU32;
		use std::sync::atomic::Ordering;

		static CALL_COUNT: AtomicU32 = AtomicU32::new(0);
		static POOL: LazyPool<u32, String, String> = LazyPool::new(|key| {
			Box::pin(async move {
				CALL_COUNT.fetch_add(1, Ordering::SeqCst);
				format!("Value for key {}", key)
			})
		});
		// Sequential calls to the same key should only construct once
		POOL.get(&1).await;
		POOL.get(&1).await;
		POOL.get(&1).await;
		CALL_COUNT.load(Ordering::SeqCst).xpect_eq(1);
	}

	#[crate::test]
	async fn try_get_works() {
		static POOL: LazyPool<u32, String, Result<String>> =
			LazyPool::new(|key| {
				Box::pin(async move { format!("Value for key {}", key).xok() })
			});
		let value = POOL.try_get(&42).await.unwrap();
		assert_eq!(value, "Value for key 42");
	}
}
