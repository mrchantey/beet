//! A secrets document over any [`BlobStore`].

use crate::prelude::*;
use beet_core::prelude::*;

/// A secrets document at a path in a store: the file behind every `secrets`
/// verb and the `<Secrets>` load, read and written typed in the format its
/// extension names. The index is plaintext, so a read needs no identity;
/// [`SecretsDocument::open`] takes one.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let mut identities = AgeIdentityFile::default();
/// identities.push(AgeIdentity::generate());
/// let handle = SecretsHandle::new(BlobStore::temp(), "secrets.toml")?;
/// let mut document = handle.read_or_new().await?;
/// document.set(&identities, "TOKEN", "hunter2", default())?;
/// handle.write(&document).await?;
/// handle
/// 	.read()
/// 	.await?
/// 	.open(&identities)?
/// 	.get("TOKEN")
/// 	.unwrap()
/// 	.value
/// 	.as_str()
/// 	.xpect_eq("hunter2");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SecretsHandle {
	/// The store the document lives in.
	pub store: BlobStore,
	/// The document within the store, its extension naming the format.
	pub path: RelPath,
	/// The `<Secrets label>` this handle resolved from, for messages.
	pub label: Option<SmolStr>,
}

impl SecretsHandle {
	/// The document at `path` in `store`. Errors on a path naming no
	/// document format.
	pub fn new(store: BlobStore, path: impl AsRef<str>) -> Result<Self> {
		SecretsDocument::media_type_of(path.as_ref())?;
		Self {
			store,
			path: RelPath::new(path),
			label: None,
		}
		.xok()
	}

	/// The document a filesystem path or store uri names, see
	/// [`BlobStore::from_file_uri`]: `~/personal.toml`,
	/// `s3://bucket/secrets/x.toml`, or a bare `x.toml` in the cwd.
	pub fn from_uri(path: &str) -> Result<Self> {
		let (store, file) = BlobStore::from_file_uri(path)?;
		Self::new(store, file)
	}

	/// The handle with the label it was declared under.
	pub fn with_label(mut self, label: impl Into<SmolStr>) -> Self {
		self.label = Some(label.into());
		self
	}

	/// How a log or an error names this document: its label and path when
	/// declared, its path otherwise.
	pub fn describe(&self) -> String {
		match &self.label {
			Some(label) => format!("`{label}` ({})", self.path),
			None => format!("`{}`", self.path),
		}
	}

	/// The format the path names.
	pub fn media_type(&self) -> Result<MediaType> {
		SecretsDocument::media_type_of(self.path.as_str())
	}

	/// Whether the document exists in its store.
	pub async fn exists(&self) -> Result<bool> {
		self.store.exists(&self.path).await
	}

	/// Read and parse the document. Errors when the file is missing.
	pub async fn read(&self) -> Result<SecretsDocument> {
		let bytes = self.store.get(&self.path).await.map_err(|err| {
			bevyhow!("document {} cannot be read: {err}", self.describe())
		})?;
		SecretsDocument::parse(self.media_type()?, &bytes)
			.map_err(|err| bevyhow!("document {}: {err}", self.describe()))
	}

	/// [`read`](Self::read), or an empty document when the file does not
	/// exist yet: the state a first `set` starts from.
	pub async fn read_or_new(&self) -> Result<SecretsDocument> {
		match self.exists().await? {
			true => self.read().await,
			false => SecretsDocument::new(self.media_type()?).xok(),
		}
	}

	/// Write `document` to this path in its format.
	pub async fn write(&self, document: &SecretsDocument) -> Result<()> {
		self.store.insert(&self.path, document.to_bytes()?).await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn reads_and_writes_a_document() {
		let mut identities = AgeIdentityFile::default();
		identities.push(AgeIdentity::generate());
		let store = BlobStore::temp();
		let handle = SecretsHandle::new(store.clone(), "secrets.toml").unwrap();
		handle.exists().await.unwrap().xpect_false();
		handle
			.read()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be read");
		let mut document = handle.read_or_new().await.unwrap();
		document.media_type().xpect_eq(MediaType::Toml);
		document.set(&identities, "A", "1", default()).unwrap();
		handle.write(&document).await.unwrap();
		// the index is plaintext toml
		String::from_utf8(
			store
				.get(&RelPath::new("secrets.toml"))
				.await
				.unwrap()
				.to_vec(),
		)
		.unwrap()
		.xpect_contains("[secrets.A]");
		handle
			.read()
			.await
			.unwrap()
			.open(&identities)
			.unwrap()
			.get("A")
			.unwrap()
			.value
			.as_str()
			.xpect_eq("1");
		SecretsHandle::new(store, "cert.pem.age")
			.unwrap_err()
			.to_string()
			.xpect_contains("not a secrets document");
	}

	#[beet_core::test]
	fn resolves_a_path_or_uri() {
		let handle =
			SecretsHandle::from_uri("memory://documents/dir/x.json").unwrap();
		handle.path.as_str().xpect_eq("x.json");
		handle.store.subdir().as_str().xpect_eq("dir");
		handle.describe().xpect_eq("`x.json`");
		SecretsHandle::from_uri("dir/x.age").unwrap_err();
	}
}
