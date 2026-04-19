use crate::prelude::*;
use base64::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use js_sys::wasm_bindgen::JsCast;

/// A bucket provider backed by browser localStorage.
///
/// Uses `bucket:<bucket_name>:<path>` as the localStorage key prefix.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = Bucket::on_add::<Self>)]
pub struct LocalStorageBucket {
	/// The bucket name used as part of the localStorage key prefix.
	bucket_name: SmolStr,
	/// Optional subdirectory prefix for all keys.
	subdir: Option<RelPath>,
}

impl LocalStorageBucket {
	/// Creates a new localStorage-backed bucket provider for the given bucket.
	pub fn new(bucket_name: impl Into<SmolStr>) -> Self {
		Self {
			bucket_name: bucket_name.into(),
			subdir: None,
		}
	}

	/// Set the subdirectory prefix for all keys.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}

	fn bucket_prefix(&self) -> String {
		format!("bucket:{}:", self.bucket_name)
	}

	/// Compose the localStorage key for the bucket and path.
	fn storage_key(&self, path: &RelPath) -> String {
		let effective = match &self.subdir {
			Some(sub) => format!("{}/{}", sub, path),
			None => path.to_string(),
		};
		self.bucket_prefix().xtend(&effective)
	}

	fn local_storage() -> web_sys::Storage {
		// deno also has localstorage so perform unchecked cast
		js_sys::global()
			.unchecked_into::<web_sys::Window>()
			.local_storage()
			.expect("LocalStorage is not available")
			.expect("Failed to access localStorage")
	}

	/// Create a [`TypedBlob`] handle for a single object in this bucket.
	pub fn blob(&self, path: RelPath) -> TypedBlob<Self> {
		TypedBlob::new(self.clone(), path)
	}
}

#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for LocalStorageBucket {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}

impl BucketProvider for LocalStorageBucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		Box::new(LocalStorageBucket {
			bucket_name: self.bucket_name.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let prefix = self.bucket_prefix();
		Box::pin(async move {
			let storage = Self::local_storage();
			for i in 0..storage.length().unwrap_or(0) {
				if let Some(key) = storage.key(i).ok().flatten() {
					if key.starts_with(&prefix) {
						return true.xok();
					}
				}
			}
			false.xok()
		})
	}

	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		// No-op for localStorage
		Box::pin(async { ().xok() })
	}

	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		let prefix = self.bucket_prefix();
		Box::pin(async move {
			let storage = Self::local_storage();
			let keys: Vec<String> = (0..storage.length().unwrap_or(0))
				.filter_map(|i| storage.key(i).ok().flatten())
				.filter(|key| key.starts_with(&prefix))
				.collect();
			for key in keys {
				storage.remove_item(&key).ok();
			}
			().xok()
		})
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let key = self.storage_key(path);
		let value = BASE64_STANDARD.encode(body);
		Box::pin(async move {
			Self::local_storage().set_item(&key, &value).map_jserr()?;
			().xok()
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let key = self.storage_key(path);
		Box::pin(async move {
			let storage = Self::local_storage();
			let value = storage.get_item(&key).map_jserr()?;
			value.is_some().xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let prefix = self.bucket_prefix();
		let subdir_prefix = self.subdir.as_ref().map(|s| format!("{}/", s));
		Box::pin(async move {
			let storage = Self::local_storage();
			let keys: Vec<RelPath> = (0..storage.length().unwrap_or(0))
				.filter_map(|i| storage.key(i).ok().flatten())
				.filter_map(|key| {
					let raw = key.strip_prefix(&prefix)?;
					let rel = match &subdir_prefix {
						Some(p) => raw.strip_prefix(p.as_str())?,
						None => raw,
					};
					Some(RelPath::new(rel))
				})
				.collect();
			keys.xok()
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let key = self.storage_key(path);
		Box::pin(async move {
			let value = Self::local_storage().get_item(&key).map_jserr()?;
			match value {
				Some(val) => {
					let bytes = BASE64_STANDARD.decode(val)?;
					Bytes::from(bytes).xok()
				}
				None => bevybail!("Object not found: {}", key),
			}
		})
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let key = self.storage_key(path);
		let this = self.clone();
		let path = path.clone();
		Box::pin(async move {
			match this.exists(&path).await? {
				true => {
					Self::local_storage().remove_item(&key).map_jserr()?;
					().xok()
				}
				false => {
					bevybail!("Object not found: {}", key)
				}
			}
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
		let provider = LocalStorageBucket::new("test-bucket");
		bucket_test::run(provider).await;
	}
}
