use crate::prelude::*;
use async_lock::RwLock;

/// A simple pool that lazily initializes values on demand,
/// with support for both verbatim and [`Result`] constructors
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
	pub async fn get(&self, key: &K) -> V {
		if let Some(value) = self.map.read().await.get(key) {
			return value.clone();
		}
		let value = (self.constructor)(key).await;
		self.map.write().await.insert(key.clone(), value.clone());
		value
	}
}

impl<K, V> LazyPool<K, V, Result<V>>
where
	K: std::hash::Hash + Eq + Clone,
	V: Clone,
{
	/// Gets the value for the given key, initializing it if it doesn't exist.
	pub async fn try_get(&self, key: &K) -> Result<V> {
		if let Some(value) = self.map.read().await.get(key) {
			return value.clone().xok();
		}
		let value = (self.constructor)(key).await?;
		self.map.write().await.insert(key.clone(), value.clone());
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
}
