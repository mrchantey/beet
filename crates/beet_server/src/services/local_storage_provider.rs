use crate::prelude::*;
use base64::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use js_sys::wasm_bindgen::JsCast;
use std::future::Future;
use std::pin::Pin;

/// A bucket provider backed by browser localStorage.
#[derive(Debug, Clone)]
pub struct LocalStorageProvider;

impl LocalStorageProvider {
	pub fn new() -> Self { Self }

	/// Compose the localStorage key for a given bucket and path.
	fn storage_key(bucket_name: &str, path: &RoutePath) -> String {
		format!("bucket:{}:{}", bucket_name, path.to_string())
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
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
		let prefix = format!("bucket:{}:", bucket_name);
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

	fn create_bucket(
		&self,
		_bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		// No-op for localStorage
		Box::pin(async { Ok(()) })
	}

	fn delete_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let prefix = format!("bucket:{}:", bucket_name);
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
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let key = Self::storage_key(bucket_name, path);
		let value = BASE64_STANDARD.encode(body);
		Box::pin(async move {
			let storage = Self::local_storage();
			storage.set_item(&key, &value).map_jserr()?;
			Ok(())
		})
	}

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>> {
		let key = Self::storage_key(bucket_name, path);
		Box::pin(async move {
			let storage = Self::local_storage();
			let value = storage.get_item(&key).map_jserr()?;
			match value {
				Some(val) => {
					let bytes = BASE64_STANDARD.decode(val)?;
					Ok(Bytes::from(bytes))
				}
				None => bevybail!("Item not found in localStorage: {}", key),
			}
		})
	}

	fn delete(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let key = Self::storage_key(bucket_name, path);
		Box::pin(async move {
			let storage = Self::local_storage();
			storage.remove_item(&key).map_jserr()?;
			Ok(())
		})
	}

	fn public_url(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>> {
		let key = Self::storage_key(bucket_name, path);
		Box::pin(async move { Ok(format!("localstorage://{}", key)) })
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let provider = LocalStorageProvider::new();
		let bucket = Bucket::new(provider, "test-bucket");
		let path = RoutePath::from("/test_path");
		let body = bytes::Bytes::from("test_body");
		bucket.exists().await.unwrap().xpect().to_be_false();
		bucket.insert(&path, body.clone()).await.unwrap();
		bucket.exists().await.unwrap().xpect().to_be_true();
		bucket.get(&path).await.unwrap().xpect().to_be(body.clone());
		bucket.get(&path).await.unwrap().xpect().to_be(body);

		bucket.delete(&path).await.unwrap();
		bucket.get(&path).await.xpect().to_be_err();

		bucket.remove().await.unwrap();
		bucket.exists().await.unwrap().xpect().to_be_false();
	}
}
