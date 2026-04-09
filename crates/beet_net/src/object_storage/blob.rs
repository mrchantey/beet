use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A handle to a single object in a bucket, identified by its [`RelPath`].
///
/// Unlike [`Bucket`] methods which require passing a path for every operation,
/// a [`Blob`] captures the path once and exposes the same per-object operations
/// without repeating it. The underlying provider can change (S3, filesystem,
/// memory, etc.) while the blob's path stays fixed.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let bucket = temp_bucket();
/// let blob = bucket.blob(RelPath::new("my-file.txt"));
/// blob.insert("hello world").await?;
/// let data = blob.get().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Get)]
pub struct Blob {
	/// Path to the blob within the bucket.
	path: RelPath,
	/// Provider that handles storage operations.
	provider: Box<dyn BucketProvider>,
}

impl std::fmt::Debug for Blob {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Blob")
			.field("path", &self.path)
			.finish_non_exhaustive()
	}
}

impl Clone for Blob {
	fn clone(&self) -> Self {
		Self {
			path: self.path.clone(),
			provider: self.provider.box_clone(),
		}
	}
}

impl Blob {
	/// Create a new [`Blob`] from a provider and path.
	pub fn new(provider: Box<dyn BucketProvider>, path: RelPath) -> Self {
		Self { path, provider }
	}

	/// Insert (or overwrite) the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = temp_bucket().blob(RelPath::new("doc.txt"));
	/// blob.insert("content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(&self, body: impl Into<Bytes>) -> Result {
		self.provider.insert(&self.path, body.into()).await
	}

	/// Insert the blob's content, failing if it already exists.
	pub async fn try_insert(&self, body: impl Into<Bytes>) -> Result {
		if self.exists().await? {
			bevybail!("Object already exists: {}", self.path)
		} else {
			self.insert(body).await
		}
	}

	/// Retrieve the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = temp_bucket().blob(RelPath::new("doc.txt"));
	/// blob.insert("hello").await?;
	/// let data = blob.get().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(&self) -> Result<Bytes> {
		self.provider.get(&self.path).await
	}

	/// Check whether the blob exists in the bucket.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = temp_bucket().blob(RelPath::new("doc.txt"));
	/// let exists = blob.exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self) -> Result<bool> {
		self.provider.exists(&self.path).await
	}

	/// Remove the blob from the bucket.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = temp_bucket().blob(RelPath::new("doc.txt"));
	/// blob.insert("temp").await?;
	/// blob.remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove(&self) -> Result {
		self.provider.remove(&self.path).await
	}

	/// Get the public URL of the blob, if the provider supports it.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = temp_bucket().blob(RelPath::new("doc.txt"));
	/// if let Some(url) = blob.public_url().await? {
	///     println!("Public URL: {url}");
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn public_url(&self) -> Result<Option<String>> {
		self.provider.public_url(&self.path).await
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn blob_from_bucket() {
		let bucket = temp_bucket();
		let blob = bucket.blob(RelPath::new("test.txt"));
		blob.path().to_string().xpect_eq("test.txt");
	}

	#[test]
	fn clone_preserves_path() {
		let blob = temp_bucket().blob(RelPath::new("a/b/c.txt"));
		let cloned = blob.clone();
		cloned.path().xpect_eq(blob.path().clone());
	}

	#[test]
	fn blob_from_provider_trait() {
		let provider = InMemoryProvider::created();
		let blob = provider.blob(RelPath::new("key.dat"));
		blob.path().to_string().xpect_eq("key.dat");
	}

	#[test]
	fn insert_get_remove() {
		async_ext::block_on(async {
			let blob = temp_bucket().blob(RelPath::new("hello.txt"));
			blob.exists().await.unwrap().xpect_false();
			blob.insert("world").await.unwrap();
			blob.exists().await.unwrap().xpect_true();
			blob.get()
				.await
				.unwrap()
				.xpect_eq(bytes::Bytes::from("world"));
			blob.remove().await.unwrap();
			blob.exists().await.unwrap().xpect_false();
		});
	}

	#[test]
	fn try_insert_fails_if_exists() {
		async_ext::block_on(async {
			let blob = temp_bucket().blob(RelPath::new("once.txt"));
			blob.insert("first").await.unwrap();
			blob.try_insert("second").await.xpect_err();
		});
	}
}
