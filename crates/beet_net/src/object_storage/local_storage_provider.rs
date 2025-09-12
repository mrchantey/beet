use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use js_sys::wasm_bindgen::JsCast;

/// A bucket provider backed by browser localStorage.
#[derive(Debug, Clone)]
pub struct LocalStorageProvider;

impl LocalStorageProvider {
	pub fn new() -> Self { Self }

	fn bucket_prefix(bucket_name: &str) -> String {
		format!("bucket:{}:", bucket_name)
	}

	/// Compose the localStorage key for a given bucket and path.
	fn storage_key(bucket_name: &str, path: &RoutePath) -> String {
		Self::bucket_prefix(bucket_name).xtend(&path.to_string())
	}

	fn local_storage() -> web_sys::Storage {
		// deno also has localstorage so perform unchecked cast
		js_sys::global()
			.unchecked_into::<web_sys::Window>()
			.local_storage()
			.expect("LocalStorage is not available")
			.expect("Failed to access localStorage")
	}
}

impl BucketProvider for LocalStorageProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<bool>> {
		let prefix = Self::bucket_prefix(bucket_name);
		Box::pin(async move {
			let storage = Self::local_storage();
			for i in 0..storage.length().unwrap_or(0) {
				if let Some(key) = storage.key(i).ok().flatten() {
					if key.starts_with(&prefix) {
						return Ok(true);
					}
				}
			}
			Ok(false)
		})
	}

	fn bucket_create(&self, _bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		// No-op for localStorage
		Box::pin(async { Ok(()) })
	}

	fn bucket_remove(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let prefix = Self::bucket_prefix(bucket_name);
		Box::pin(async move {
			let storage = Self::local_storage();
			let keys: Vec<String> = (0..storage.length().unwrap_or(0))
				.filter_map(|i| storage.key(i).ok().flatten())
				.filter(|k| k.starts_with(&prefix))
				.collect();
			for key in keys {
				storage.remove_item(&key).ok();
			}
			Ok(())
		})
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result<()>> {
		let key = Self::storage_key(bucket_name, path);
		let value = BASE64_STANDARD.encode(body);
		Box::pin(async move {
			Self::local_storage().set_item(&key, &value).map_jserr()?;
			Ok(())
		})
	}

	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>> {
		let key = Self::storage_key(bucket_name, path);
		Box::pin(async move {
			let storage = Self::local_storage();
			let value = storage.get_item(&key).map_jserr()?;
			Ok(value.is_some())
		})
	}

	fn list(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let prefix = Self::bucket_prefix(bucket_name);
		Box::pin(async move {
			let storage = Self::local_storage();
			let keys: Vec<RoutePath> = (0..storage.length().unwrap_or(0))
				.filter_map(|i| storage.key(i).ok().flatten())
				.filter_map(|k| k.strip_prefix(&prefix).map(RoutePath::new))
				.collect();
			Ok(keys)
		})
	}

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>> {
		let key = Self::storage_key(bucket_name, path);
		Box::pin(async move {
			let value = Self::local_storage().get_item(&key).map_jserr()?;
			match value {
				Some(val) => {
					let bytes = BASE64_STANDARD.decode(val)?;
					Ok(Bytes::from(bytes))
				}
				None => bevybail!("Object not found: {}", key),
			}
		})
	}

	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<()>> {
		let key = Self::storage_key(bucket_name, path);
		let this = self.clone();
		let bucket_name = bucket_name.to_string();
		let path = path.clone();

		Box::pin(async move {
			match this.exists(&bucket_name, &path).await? {
				true => {
					Self::local_storage().remove_item(&key).map_jserr()?;
					Ok(())
				}
				false => {
					bevybail!("Object not found: {}", key)
				}
			}
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
		let provider = LocalStorageProvider::new();
		bucket_test::run(provider).await;
	}
}
