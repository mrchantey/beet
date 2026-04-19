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
/// let bucket = Bucket::temp();
/// let blob = bucket.blob(RelPath::new("my-file.txt"));
/// blob.insert("hello world").await?;
/// let data = blob.get().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Component, Get, Deref)]
pub struct Blob {
	/// Path to the blob within the bucket.
	#[deref]
	path: RelPath,
	/// Provider that handles storage operations.
	bucket: Bucket,
}

impl Blob {
	/// Create a new [`Blob`] from a provider and path.
	pub fn new(bucket: Bucket, path: RelPath) -> Self { Self { path, bucket } }

	/// Insert (or overwrite) the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// blob.insert("content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(&self, body: impl Into<Bytes>) -> Result {
		self.bucket.insert(&self.path, body.into()).await
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
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// blob.insert("hello").await?;
	/// let data = blob.get().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(&self) -> Result<Bytes> {
		self.bucket.get(&self.path).await
	}

	/// Retrieve the blob's content as [`MediaBytes`], inferring the
	/// [`MediaType`] from the path extension.
	pub async fn get_media(&self) -> Result<MediaBytes> {
		let media_type = MediaType::from_path(self.path.as_path());
		let bytes = self.get().await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Check whether the blob exists in the bucket.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// let exists = blob.exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self) -> Result<bool> {
		self.bucket.exists(&self.path).await
	}

	/// Remove the blob from the bucket.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// blob.insert("temp").await?;
	/// blob.remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove(&self) -> Result {
		self.bucket.remove(&self.path).await
	}

	/// Get the public URL of the blob, if the provider supports it.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// if let Some(url) = blob.public_url().await? {
	///     println!("Public URL: {url}");
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn public_url(&self) -> Result<Option<String>> {
		self.bucket.public_url(&self.path).await
	}
}


/// Serializable blob handle that inserts an erased [`Blob`] on add.
///
/// Unlike [`Blob`], this type is fully reflectable and can be used in
/// Bevy scenes. When added to an entity the `on_add` hook automatically
/// inserts a [`Blob`] component so that systems reading [`Blob`] continue
/// to work unchanged.
///
/// # Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let typed = FsBucket::new(
///     AbsPathBuf::new_workspace_rel("my_dir").unwrap()
/// ).blob(RelPath::new("file.txt"));
/// // `typed` is a `TypedBlob<FsBucket>` — reflectable and serializable.
/// ```
#[derive(Clone, Component, Reflect, Get)]
#[reflect(Component)]
#[component(on_add = on_add_typed_blob::<B>)]
pub struct TypedBlob<B>
where
	B: 'static + Send + Sync + Clone + Reflect + BucketProvider,
{
	/// Path to the blob within the bucket.
	path: RelPath,
	/// Typed bucket that owns this blob.
	#[get(skip)]
	bucket: B,
}

fn on_add_typed_blob<B>(mut world: DeferredWorld, cx: HookContext)
where
	B: 'static + Send + Sync + Clone + Reflect + BucketProvider,
{
	let blob = world
		.entity(cx.entity)
		.get::<TypedBlob<B>>()
		.unwrap()
		.clone()
		.to_blob();
	world.commands().entity(cx.entity).insert(blob);
}

impl<B> TypedBlob<B>
where
	B: 'static + Send + Sync + Clone + Reflect + BucketProvider,
{
	/// Create a new [`TypedBlob`] from a [`TypedBucket`] and path.
	pub fn new(bucket: B, path: RelPath) -> Self { Self { path, bucket } }

	/// Convert to an erased [`Blob`].
	pub fn to_blob(&self) -> Blob {
		Blob::new(Bucket::new(self.bucket.clone()), self.path.clone())
	}

	/// Get the underlying [`TypedBucket`].
	pub fn bucket(&self) -> &B { &self.bucket }

	/// Get the underlying [`Bucket`] (type-erased).
	pub fn erased_bucket(&self) -> Bucket { Bucket::new(self.bucket.clone()) }

	/// Insert (or overwrite) the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// blob.insert("content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(&self, body: impl Into<Bytes>) -> Result {
		self.bucket.insert(&self.path, body.into()).await
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
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// blob.insert("hello").await?;
	/// let data = blob.get().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(&self) -> Result<Bytes> {
		self.bucket.get(&self.path).await
	}

	/// Retrieve the blob's content as [`MediaBytes`], inferring the
	/// [`MediaType`] from the path extension.
	pub async fn get_media(&self) -> Result<MediaBytes> {
		let media_type = MediaType::from_path(self.path.as_path());
		let bytes = self.get().await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Check whether the blob exists in the bucket.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// let exists = blob.exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self) -> Result<bool> {
		self.bucket.exists(&self.path).await
	}

	/// Remove the blob from the bucket.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// blob.insert("temp").await?;
	/// blob.remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove(&self) -> Result {
		self.bucket.remove(&self.path).await
	}

	/// Get the public URL of the blob, if the provider supports it.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = Bucket::temp().blob(RelPath::new("doc.txt"));
	/// if let Some(url) = blob.public_url().await? {
	///     println!("Public URL: {url}");
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn public_url(&self) -> Result<Option<String>> {
		self.bucket.public_url(&self.path).await
	}
}

impl<B> std::fmt::Debug for TypedBlob<B>
where
	B: 'static
		+ Send
		+ Sync
		+ Clone
		+ Reflect
		+ BucketProvider
		+ std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("TypedBlob")
			.field("path", &self.path)
			.field("bucket", &self.bucket)
			.finish()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn blob_from_bucket() {
		let bucket = Bucket::temp();
		let blob = bucket.blob(RelPath::new("test.txt"));
		blob.path().to_string().xpect_eq("test.txt");
	}

	#[test]
	fn clone_preserves_path() {
		let blob = Bucket::temp().blob(RelPath::new("a/b/c.txt"));
		let cloned = blob.clone();
		cloned.path().xpect_eq(blob.path().clone());
	}

	#[test]
	fn blob_from_provider_trait() {
		let provider = InMemoryBucket::new();
		let blob = provider.erased_blob(RelPath::new("key.dat"));
		blob.path().to_string().xpect_eq("key.dat");
	}

	#[test]
	fn typed_blob_to_blob() {
		let bucket = FsBucket::new(
			AbsPathBuf::new_workspace_rel("target/tests/typed_blob").unwrap(),
		);
		let typed = bucket.blob(RelPath::new("test.txt"));
		typed.path().to_string().xpect_eq("test.txt");
		let erased = typed.to_blob();
		erased.path().to_string().xpect_eq("test.txt");
	}

	#[test]
	fn insert_get_remove() {
		async_ext::block_on(async {
			let blob = Bucket::temp().blob(RelPath::new("hello.txt"));
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
	fn get_media_infers_type() {
		async_ext::block_on(async {
			let blob = Bucket::temp().blob(RelPath::new("data.json"));
			blob.insert(r#"{"key":"value"}"#).await.unwrap();
			let media = blob.get_media().await.unwrap();
			media.media_type().xpect_eq(MediaType::Json);
			media.as_utf8().unwrap().xpect_contains("key");
		});
	}

	#[test]
	fn try_insert_fails_if_exists() {
		async_ext::block_on(async {
			let blob = Bucket::temp().blob(RelPath::new("once.txt"));
			blob.insert("first").await.unwrap();
			blob.try_insert("second").await.xpect_err();
		});
	}
}
