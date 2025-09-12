use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;


#[derive(Debug, Clone)]
pub struct FsBucketProvider {
	/// The root path for the filesystem bucket provider,
	/// all created buckets will be under this path.
	/// For example, if the root is `/data/buckets` and the bucket name is
	/// `my-bucket`, the bucket will be at `/data/buckets/my-bucket`.
	root: AbsPathBuf,
}

impl FsBucketProvider {
	/// Create a new filesystem bucket provider with the given root path
	pub fn new(root: impl Into<AbsPathBuf>) -> Self {
		Self { root: root.into() }
	}
	/// Resolve the path for a bucket and key, handling leading slashes.
	fn resolve_path(&self, bucket_name: &str, path: &RoutePath) -> AbsPathBuf {
		self.root
			.join(bucket_name)
			.join(path.to_string().trim_start_matches('/'))
	}
}

impl BucketProvider for FsBucketProvider {
	fn box_clone(&self) -> Box<dyn BucketProvider> { Box::new(self.clone()) }

	fn region(&self) -> Option<String> { None }

	fn bucket_exists(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<bool>> {
		let path = self.root.join(bucket_name);
		Box::pin(async move { tokio::fs::try_exists(path).await?.xok() })
	}

	fn bucket_create(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let path = self.root.join(bucket_name);
		Box::pin(async move {
			tokio::fs::create_dir_all(path).await?;
			Ok(())
		})
	}

	fn bucket_remove(&self, bucket_name: &str) -> SendBoxedFuture<Result<()>> {
		let path = self.root.join(bucket_name);
		Box::pin(async move {
			FsExt::remove_async(path).await?;
			Ok(())
		})
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> SendBoxedFuture<Result<()>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move {
			FsExt::write_async(path, body).await?;
			Ok(())
		})
	}

	fn list(
		&self,
		bucket_name: &str,
	) -> SendBoxedFuture<Result<Vec<RoutePath>>> {
		let bucket_path = self.root.join(bucket_name);
		Box::pin(async move {
			ReadDir::files_recursive_async(&bucket_path)
				.await?
				.into_iter()
				.map(|path| {
					let path = path
						.strip_prefix(&bucket_path)
						.unwrap_or_else(|_| path.as_path());
					RoutePath::new(path)
				})
				.collect::<Vec<_>>()
				.xok()
		})
	}
	fn exists(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<bool>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move { tokio::fs::try_exists(path).await?.xok() })
	}

	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<Bytes>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move {
			ReadFile::to_bytes_async(&path)
				.await
				.map_err(|_| HttpError::not_found())?
				.xmap(Bytes::from)
				.xok()
		})
	}
	fn remove(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> SendBoxedFuture<Result<()>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move {
			tokio::fs::remove_file(path).await?;
			Ok(())
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
	use beet_core::prelude::*;

	#[sweet::test]
	async fn works() {
		let dir = "target/test_buckets/test-bucket-001";
		let provider =
			FsBucketProvider::new(AbsPathBuf::new_workspace_rel(dir).unwrap());
		bucket_test::run(provider).await;
	}
}
