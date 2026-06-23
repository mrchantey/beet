use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A handle to a single object in a store, identified by its [`SmolPath`].
///
/// Unlike [`BlobStore`] methods which require passing a path for every operation,
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
/// let store = BlobStore::temp();
/// let blob = store.blob(SmolPath::new("my-file.txt"));
/// blob.insert("hello world").await?;
/// let data = blob.get().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Component, Get, Deref)]
pub struct Blob {
	/// Path to the blob within the store.
	#[deref]
	path: SmolPath,
	/// Provider that handles storage operations.
	store: BlobStore,
}

impl Blob {
	/// Create a new [`Blob`] from a provider and path.
	pub fn new(store: BlobStore, path: SmolPath) -> Self {
		Self { path, store }
	}

	/// Whether `other` targets the same object: same backing store, subdir, and
	/// path. The [`BlobPath`] change-detection uses it to skip a no-op re-resolve.
	pub fn same_target(&self, other: &Blob) -> bool {
		self.path == other.path && self.store.same_scope(&other.store)
	}

	/// True if `event` is this exact object: same backing and the
	/// root-relative locations are equal (object-exact, not scope-covering).
	pub fn matches_event(&self, event: &BlobEvent) -> bool {
		self.store.root_key() == event.store.root_key()
			&& self.store.subdir().join(&self.path)
				== event.root_relative_path()
	}

	/// Insert (or overwrite) the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// blob.insert("content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(&self, body: impl Into<Bytes>) -> Result {
		self.store.insert(&self.path, body.into()).await
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
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// blob.insert("hello").await?;
	/// let data = blob.get().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(&self) -> Result<Bytes> {
		self.store.get(&self.path).await
	}

	/// Retrieve the blob's content as [`MediaBytes`], inferring the
	/// [`MediaType`] from the path extension.
	pub async fn get_media(&self) -> Result<MediaBytes> {
		let media_type = self.path.media_type().unwrap_or(MediaType::Bytes);
		let bytes = self.get().await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Check whether the blob exists in the store.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// let exists = blob.exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self) -> Result<bool> {
		self.store.exists(&self.path).await
	}

	/// Remove the blob from the store.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// blob.insert("temp").await?;
	/// blob.remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove(&self) -> Result { self.store.remove(&self.path).await }

	/// Get the public URL of the blob, if the provider supports it.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// if let Some(url) = blob.public_url().await? {
	///     println!("Public URL: {url}");
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn public_url(&self) -> Result<Option<String>> {
		self.store.public_url(&self.path).await
	}
}

/// Serializable blob handle that inserts an erased [`Blob`] on add.
///
/// Unlike [`Blob`], this type is fully reflectable and can be used in
/// world serialization. When added to an entity the `on_add` hook automatically
/// inserts a [`Blob`] component so that systems reading [`Blob`] continue
/// to work unchanged.
///
/// # Example
///
/// ```no_run
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let typed = FsStore::new(
///     AbsPathBuf::new_workspace_rel("my_dir").unwrap()
/// ).blob(SmolPath::new("file.txt"));
/// // `typed` is a `TypedBlob<FsStore>` — reflectable and serializable.
/// ```
#[derive(Clone, Component, Reflect, Get)]
#[reflect(Component)]
#[component(on_add = on_add_typed_blob::<B>)]
pub struct TypedBlob<B>
where
	B: 'static + Send + Sync + Clone + Reflect + BlobStoreProvider,
{
	/// Path to the blob within the store.
	path: SmolPath,
	/// Typed store that owns this blob.
	#[get(skip)]
	store: B,
}

fn on_add_typed_blob<B>(mut world: DeferredWorld, cx: HookContext)
where
	B: 'static + Send + Sync + Clone + Reflect + BlobStoreProvider,
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
	B: 'static + Send + Sync + Clone + Reflect + BlobStoreProvider,
{
	/// Create a new [`TypedBlob`] from a typed store and path.
	pub fn new(store: B, path: SmolPath) -> Self { Self { path, store } }

	/// Convert to an erased [`Blob`].
	pub fn to_blob(&self) -> Blob {
		Blob::new(BlobStore::new(self.store.clone()), self.path.clone())
	}

	/// Get the underlying typed store.
	pub fn store(&self) -> &B { &self.store }

	/// Get the underlying [`BlobStore`] (type-erased).
	pub fn erased_store(&self) -> BlobStore {
		BlobStore::new(self.store.clone())
	}

	/// Insert (or overwrite) the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// blob.insert("content").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn insert(&self, body: impl Into<Bytes>) -> Result {
		self.store.insert(&self.path, body.into()).await
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
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// blob.insert("hello").await?;
	/// let data = blob.get().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get(&self) -> Result<Bytes> {
		self.store.get(&self.path).await
	}

	/// Retrieve the blob's content as [`MediaBytes`], inferring the
	/// [`MediaType`] from the path extension.
	pub async fn get_media(&self) -> Result<MediaBytes> {
		let media_type = self.path.media_type().unwrap_or(MediaType::Bytes);
		let bytes = self.get().await?;
		Ok(MediaBytes::new(media_type, bytes.to_vec()))
	}

	/// Check whether the blob exists in the store.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// let exists = blob.exists().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exists(&self) -> Result<bool> {
		self.store.exists(&self.path).await
	}

	/// Remove the blob from the store.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// blob.insert("temp").await?;
	/// blob.remove().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn remove(&self) -> Result { self.store.remove(&self.path).await }

	/// Get the public URL of the blob, if the provider supports it.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(SmolPath::new("doc.txt"));
	/// if let Some(url) = blob.public_url().await? {
	///     println!("Public URL: {url}");
	/// }
	/// # Ok(())
	/// # }
	/// ```
	pub async fn public_url(&self) -> Result<Option<String>> {
		self.store.public_url(&self.path).await
	}
}

impl<B> core::fmt::Debug for TypedBlob<B>
where
	B: 'static
		+ Send
		+ Sync
		+ Clone
		+ Reflect
		+ BlobStoreProvider
		+ core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("TypedBlob")
			.field("path", &self.path)
			.field("store", &self.store)
			.finish()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn blob_from_store() {
		let store = BlobStore::temp();
		let blob = store.blob(SmolPath::new("test.txt"));
		blob.path().to_string().xpect_eq("test.txt");
	}

	#[beet_core::test]
	fn clone_preserves_path() {
		let blob = BlobStore::temp().blob(SmolPath::new("a/b/c.txt"));
		let cloned = blob.clone();
		cloned.path().xpect_eq(blob.path().clone());
	}

	#[beet_core::test]
	fn blob_from_provider_trait() {
		let provider = InMemoryStore::new();
		let blob = provider.erased_blob(SmolPath::new("key.dat"));
		blob.path().to_string().xpect_eq("key.dat");
	}

	#[beet_core::test]
	fn typed_blob_to_blob() {
		let store = FsStore::new(
			AbsPathBuf::new_workspace_rel("target/tests/typed_blob").unwrap(),
		);
		let typed = store.blob(SmolPath::new("test.txt"));
		typed.path().to_string().xpect_eq("test.txt");
		let erased = typed.to_blob();
		erased.path().to_string().xpect_eq("test.txt");
	}

	#[beet_core::test]
	fn insert_get_remove() {
		async_ext::block_on(async {
			let blob = BlobStore::temp().blob(SmolPath::new("hello.txt"));
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

	#[beet_core::test]
	fn get_media_infers_type() {
		async_ext::block_on(async {
			let blob = BlobStore::temp().blob(SmolPath::new("data.json"));
			blob.insert(r#"{"key":"value"}"#).await.unwrap();
			let media = blob.get_media().await.unwrap();
			media.media_type().xpect_eq(MediaType::Json);
			media.as_utf8().unwrap().xpect_contains("key");
		});
	}

	#[beet_core::test]
	fn try_insert_fails_if_exists() {
		async_ext::block_on(async {
			let blob = BlobStore::temp().blob(SmolPath::new("once.txt"));
			blob.insert("first").await.unwrap();
			blob.try_insert("second").await.xpect_err();
		});
	}
}
