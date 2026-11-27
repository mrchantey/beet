use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;

/// A handle to a single object in a store, identified by its [`FwdPath`].
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
/// let blob = store.blob(fwd_path!("my-file.txt"));
/// blob.insert("hello world").await?;
/// let data = blob.get().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Component, Get, Deref)]
pub struct Blob {
	/// Path to the blob within the store.
	#[deref]
	path: FwdPath,
	/// Provider that handles storage operations.
	store: BlobStore,
}

impl Blob {
	/// Create a new [`Blob`] from a provider and path.
	pub fn new(store: BlobStore, path: FwdPath) -> Self { Self { path, store } }

	/// Whether `other` targets the same object: same backing store, subdir, and
	/// path. The [`BlobPath`] change-detection uses it to skip a no-op re-resolve.
	pub fn same_target(&self, other: &Blob) -> bool {
		self.path == other.path && self.store.same_scope(&other.store)
	}

	/// True if `event` is this exact object (object-exact, not
	/// scope-covering), as the store answers it
	/// ([`BlobProvider::matches_object`]).
	pub fn matches_event(&self, event: &BlobEvent) -> bool {
		self.store.matches_object(event, &self.path)
	}

	/// The file this names, its extension written: the file itself, else the
	/// one file beside it of its name with an extension.
	///
	/// ## Errors
	/// Errors when no file is, or several are.
	pub async fn complete(&self) -> Result<Self> {
		if self.exists().await? {
			return self.clone().xok();
		}
		let dir = self.path.parent().unwrap_or_default();
		let name = self.path.file_name().unwrap_or_default();
		let found = self
			.store
			.list_dir(&dir)
			.await?
			.files
			.into_iter()
			.map(|file| dir.try_join(file))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.filter(|file| file.file_stem() == Some(name))
			.collect::<Vec<_>>();
		match found.as_slice() {
			[found] => self.store.blob(found.clone()).xok(),
			[] => bevybail!(
				"no document at `{}` in `{}`",
				self.path,
				self.store.describe()
			),
			found => bevybail!(
				"`{}` names several documents, write its extension: {}",
				self.path,
				found
					.iter()
					.map(|path| format!("`{path}`"))
					.collect::<Vec<_>>()
					.join(", ")
			),
		}
	}

	/// Insert (or overwrite) the blob's content.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(fwd_path!("doc.txt"));
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
	/// let blob = BlobStore::temp().blob(fwd_path!("doc.txt"));
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
		MediaBytes::new(media_type, self.get().await?).xok()
	}

	/// Check whether the blob exists in the store.
	///
	/// # Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use beet_net::prelude::*;
	/// # async fn run() -> Result<()> {
	/// let blob = BlobStore::temp().blob(fwd_path!("doc.txt"));
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
	/// let blob = BlobStore::temp().blob(fwd_path!("doc.txt"));
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
	/// let blob = BlobStore::temp().blob(fwd_path!("doc.txt"));
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn blob_from_store() {
		let store = BlobStore::temp();
		let blob = store.blob(fwd_path!("test.txt"));
		blob.path().to_string().xpect_eq("test.txt");
	}

	#[beet_core::test]
	fn clone_preserves_path() {
		let blob = BlobStore::temp().blob(fwd_path!("a/b/c.txt"));
		let cloned = blob.clone();
		cloned.path().xpect_eq(blob.path().clone());
	}

	#[beet_core::test]
	fn insert_get_remove() {
		async_ext::block_on(async {
			let blob = BlobStore::temp().blob(fwd_path!("hello.txt"));
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
			let blob = BlobStore::temp().blob(fwd_path!("data.json"));
			blob.insert(r#"{"key":"value"}"#).await.unwrap();
			let media = blob.get_media().await.unwrap();
			media.media_type().xpect_eq(MediaType::Json);
			media.as_utf8().unwrap().xpect_contains("key");
		});
	}

	#[beet_core::test]
	fn try_insert_fails_if_exists() {
		async_ext::block_on(async {
			let blob = BlobStore::temp().blob(fwd_path!("once.txt"));
			blob.insert("first").await.unwrap();
			blob.try_insert("second").await.xpect_err();
		});
	}
}
