use crate::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// remove a file on drop
pub struct RemoveOnDrop {
	path: AbsPathBuf,
}
impl RemoveOnDrop {
	pub fn new(path: AbsPathBuf) -> Self { Self { path } }
}
impl Drop for RemoveOnDrop {
	fn drop(&mut self) { std::fs::remove_dir_all(&self.path).ok(); }
}

impl Bucket {
	/// create a new [`FsBucketProvider`] in target/test_buckets
	pub async fn new_test() -> (Self, RemoveOnDrop) {
		static TEST_BUCKET_COUNTER: AtomicU64 = AtomicU64::new(0);

		let count = TEST_BUCKET_COUNTER.fetch_add(1, Ordering::Relaxed);
		let name = format!("bucket-{}", count);
		let provider = FsBucketProvider::new(
			AbsPathBuf::new_workspace_rel("target/test_buckets").unwrap(),
		);
		let path = AbsPathBuf::new_workspace_rel("target/test_buckets")
			.unwrap()
			.join(&name);
		let bucket = Self::new(provider, &name);
		bucket.ensure_exists().await.unwrap();

		// bucket.insert("test.html", "<h1>Test</h1>").await.unwrap();
		(bucket, RemoveOnDrop::new(path))
	}
}


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
		bucket_name: &str,
		path: &RoutePath,
	) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'static>> {
		let path = self.resolve_path(bucket_name, path);
		Box::pin(async move {
			let public_url = format!("file://{}", path.to_string());
			Ok(public_url)
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn drops() {
		let (bucket, _) = Bucket::new_test().await;
		// immediate drop
		bucket.exists().await.unwrap().xpect().to_be_false();
		let (bucket, _drop) = Bucket::new_test().await;
		bucket.exists().await.unwrap().xpect().to_be_true();
		drop(_drop);
		bucket.exists().await.unwrap().xpect().to_be_false();
	}
	#[sweet::test]
	async fn works() {
		use bytes::Bytes;

		let (bucket, _drop) = Bucket::new_test().await;
		let path = RoutePath::from("/test_path");
		let body = Bytes::from("test_body");

		bucket.insert(&path, body.clone()).await.unwrap();
		bucket.get(&path).await.unwrap().xpect().to_be(body.clone());
		bucket
			// handles leading slashes
			.get(&path)
			.await
			.unwrap()
			.xpect()
			.to_be(body);

		bucket.delete(&path).await.unwrap();
		bucket.get(&path).await.xpect().to_be_err();

		bucket.remove().await.unwrap();
		bucket.exists().await.unwrap().xpect().to_be_false();
	}
}
