use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use std::pin::Pin;


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
	) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'static>> {
		let path = self.root.join(bucket_name);
		Box::pin(async move { Ok(path.exists()) })
	}

	fn create_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let path = self.root.join(bucket_name);
		let create_bucket_req = tokio::fs::create_dir_all(path);
		Box::pin(async move {
			create_bucket_req.await?;
			Ok(())
		})
	}

	fn delete_bucket(
		&self,
		bucket_name: &str,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let path = self.root.join(bucket_name);
		Box::pin(async move {
			tokio::fs::remove_dir_all(path).await?;
			Ok(())
		})
	}

	fn insert(
		&self,
		bucket_name: &str,
		path: &RoutePath,
		body: Bytes,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move {
			if let Some(parent) = path.parent() {
				tokio::fs::create_dir_all(parent).await?;
			}
			tokio::fs::write(path, body).await?;
			Ok(())
		})
	}
	fn get(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<Bytes>> + Send + 'static>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move {
			let body_bytes = tokio::fs::read(&path).await.map_err(|_err| {
				// error!("File not found: {}", path);
				HttpError::not_found()
			})?;
			Ok(Bytes::from(body_bytes))
		})
	}
	fn delete(
		&self,
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
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
	) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'static>>
	{
		Box::pin(async move { Ok(None) })
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[sweet::test]
	async fn works() {
		let dir = "target/test_buckets/test-bucket-001";
		let provider =
			FsBucketProvider::new(AbsPathBuf::new_workspace_rel(dir).unwrap());
		super::super::bucket_test::run(provider).await;
	}
}
