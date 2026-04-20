use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;


/// Filesystem-backed bucket for local storage.
///
/// Stores objects as files on the local filesystem, with the configured
/// path representing the full bucket directory.
///
/// ## Default
/// The default bucket is relative to the workspace root.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = Bucket::on_add::<Self>)]
pub struct FsBucket {
	/// The full path to the bucket directory.
	path: AbsPathBuf,
	/// Optional subdirectory from which all paths are resolved.
	subdir: Option<RelPath>,
}

impl Default for FsBucket {
	fn default() -> Self {
		Self {
			path: WsPathBuf::default().into(),
			subdir: None,
		}
	}
}

impl FsBucket {
	/// Create a new filesystem bucket with the given bucket path.
	pub fn new(path: impl Into<AbsPathBuf>) -> Self {
		Self {
			path: path.into(),
			subdir: None,
		}
	}
	/// Set the subdirectory from which all paths are resolved.
	pub fn with_subdir(mut self, subdir: impl Into<RelPath>) -> Self {
		self.subdir = Some(subdir.into());
		self
	}
	/// Resolve the effective root directory, including subdir if set.
	pub fn effective_root(&self) -> AbsPathBuf {
		match &self.subdir {
			Some(sub) => self.path.join(sub.to_string()),
			None => self.path.clone(),
		}
	}
	/// Resolve the full path for an object key.
	fn resolve_path(&self, route: &RelPath) -> AbsPathBuf {
		self.effective_root().join(route.to_string())
	}
	/// Create a [`TypedBlob`] handle for a single object in this bucket.
	pub fn blob(&self, path: RelPath) -> TypedBlob<Self> {
		TypedBlob::new(self.clone(), path)
	}
}

#[cfg(feature = "json")]
impl<T: TableStoreRow> TableProvider<T> for FsBucket {
	fn box_clone_table(&self) -> Box<dyn TableProvider<T>> {
		Box::new(self.clone())
	}
}


impl BucketProvider for FsBucket {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn with_subdir(&self, path: RelPath) -> Box<dyn BucketProvider> {
		Box::new(FsBucket {
			path: self.path.clone(),
			subdir: Some(match &self.subdir {
				Some(existing) => existing.join(&path),
				None => path,
			}),
		})
	}

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(&self) -> SendBoxedFuture<Result<bool>> {
		let root = self.effective_root();
		Box::pin(async move { fs_ext::exists_async(root).await?.xok() })
	}

	fn bucket_create(&self) -> SendBoxedFuture<Result> {
		let root = self.effective_root();
		Box::pin(async move {
			fs_ext::create_dir_all_async(root).await?;
			().xok()
		})
	}

	fn bucket_remove(&self) -> SendBoxedFuture<Result> {
		let root = self.effective_root();
		Box::pin(async move {
			fs_ext::remove_async(root).await?;
			().xok()
		})
	}

	fn insert(&self, path: &RelPath, body: Bytes) -> SendBoxedFuture<Result> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			fs_ext::write_async(path, body).await?;
			().xok()
		})
	}

	fn list(&self) -> SendBoxedFuture<Result<Vec<RelPath>>> {
		let root = self.effective_root();
		Box::pin(async move {
			ReadDir::files_recursive_async(&root)
				.await?
				.into_iter()
				.map(|path| {
					let path = path
						.strip_prefix(&root)
						.unwrap_or_else(|_| path.as_path());
					RelPath::new(path)
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}

	fn get(&self, path: &RelPath) -> SendBoxedFuture<Result<Bytes>> {
		let path = self.resolve_path(path);
		Box::pin(async move {
			fs_ext::read_async(&path)
				.await
				.map_err(|_| HttpError::not_found())?
				.xmap(Bytes::from)
				.xok()
		})
	}

	fn exists(&self, path: &RelPath) -> SendBoxedFuture<Result<bool>> {
		let path = self.resolve_path(path);
		Box::pin(async move { fs_ext::exists_async(path).await?.xok() })
	}

	fn remove(&self, path: &RelPath) -> SendBoxedFuture<Result> {
		let path = self.resolve_path(path);
		Box::pin(async move { fs_ext::remove_async(path).await?.xok() })
	}

	fn public_url(
		&self,
		_path: &RelPath,
	) -> SendBoxedFuture<Result<Option<String>>> {
		Box::pin(async move { Ok(None) })
	}
}


#[cfg(test)]
// TODO js_runtime fs support
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		let dir = "target/tests/beet_net/test-bucket-001";
		let provider =
			FsBucket::new(AbsPathBuf::new_workspace_rel(dir).unwrap());
		bucket_test::run(provider).await;
	}
}
