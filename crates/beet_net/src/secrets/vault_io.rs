//! Age files over any [`BlobStore`]: the byte layer every secret rides on.

use crate::prelude::*;
use beet_core::prelude::*;

/// An age file at a path in a store: the file behind `encrypt`, `decrypt`
/// and `rekey`, and the bytes a secrets document is read from and written
/// to. A read decrypts with an identity file, a write encrypts plaintext to
/// a recipient list and lands the armored age file through the store; the
/// plaintext is bytes in memory, never on disk.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # async fn run() -> Result<()> {
/// let identity = AgeIdentity::generate();
/// let mut identities = AgeIdentityFile::default();
/// identities.push(identity.clone());
/// let vault = VaultHandle::new(BlobStore::temp(), "notes.toml.age")?;
/// let plaintext = b"password = \"hunter2\"\n";
/// vault.write(plaintext, &[identity.to_recipient()]).await?;
/// vault.read(&identities).await?.xpect_eq(plaintext.to_vec());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct VaultHandle {
	/// The store the file lives in.
	pub store: BlobStore,
	/// The file within the store, ending in `.age`.
	pub path: RelPath,
}

impl VaultHandle {
	/// What every vault path ends in.
	pub const SUFFIX: &'static str = ".age";

	/// The file at `path` in `store`. Errors on a path that does not end in
	/// `.age`.
	pub fn new(store: BlobStore, path: impl AsRef<str>) -> Result<Self> {
		Self::validate_path(path.as_ref())?;
		Self {
			store,
			path: RelPath::new(path),
		}
		.xok()
	}

	/// The file a filesystem path or store uri names: the store is the file's
	/// directory (or the uri without its last segment), the file its last
	/// segment. `~/a/b.toml.age` is `fs:~/a` + `b.toml.age`,
	/// `s3://bucket/dir/b.toml.age?region=x` is `s3://bucket/dir?region=x` +
	/// `b.toml.age`, and a bare `b.toml.age` is the cwd.
	pub fn from_uri(path: &str) -> Result<Self> {
		let (uri, file) = Self::split_uri(path)?;
		let store = StoreProvider::from_uri(&uri)?.into_blob_store();
		Self::new(store, file)
	}

	/// The one rule a vault path must satisfy: it ends in `.age`, so a write
	/// never lands ciphertext under a plaintext name.
	pub fn validate_path(path: &str) -> Result<()> {
		match path.ends_with(Self::SUFFIX) && path.len() > Self::SUFFIX.len() {
			true => Ok(()),
			false => bevybail!(
				"`{path}` is not a vault: a vault ends in `{}`, ie \
				`secrets.toml.age` or `id_ed25519.age`",
				Self::SUFFIX
			),
		}
	}

	/// How a log or an error names this vault.
	pub fn describe(&self) -> String { format!("`{}`", self.path) }

	/// Whether the file exists in its store.
	pub async fn exists(&self) -> Result<bool> {
		self.store.exists(&self.path).await
	}

	/// Read and decrypt the file with whichever identity in `identities` it
	/// was encrypted to. Errors when the file is missing.
	pub async fn read(&self, identities: &AgeIdentityFile) -> Result<Vec<u8>> {
		let ciphertext = self.store.get(&self.path).await.map_err(|err| {
			bevyhow!("vault {} cannot be read: {err}", self.describe())
		})?;
		identities.decrypt(&ciphertext).map_err(|err| {
			bevyhow!("vault {} cannot be opened: {err}", self.describe())
		})
	}

	/// Encrypt `plaintext` to `recipients` and write it, as armored text.
	pub async fn write(
		&self,
		plaintext: &[u8],
		recipients: &[AgeRecipient],
	) -> Result<()> {
		let ciphertext = AgeRecipient::encrypt(recipients, plaintext)?;
		self.store.insert(&self.path, ciphertext).await
	}

	/// Split a path or uri into the store uri of its directory and its file
	/// name, see [`from_uri`](Self::from_uri).
	fn split_uri(path: &str) -> Result<(StoreUri, String)> {
		let path = path.trim();
		let (base, query) = path
			.split_once('?')
			.map(|(base, query)| (base, format!("?{query}")))
			.unwrap_or((path, String::new()));
		let is_uri = base.contains("://") || base.starts_with("fs:");
		let (dir, file) = match base.rsplit_once('/') {
			// `fs:` alone is the cwd
			Some((dir, file))
				if !dir.ends_with(':') && !dir.ends_with(":/") =>
			{
				(dir.to_string(), file)
			}
			Some(_) => bevybail!(
				"`{path}` names no file: a vault uri is a store followed by the \
				file within it, ie `s3://bucket/secrets/x.toml.age`"
			),
			None => match is_uri {
				true => ("fs".to_string(), base.trim_start_matches("fs:")),
				false => (".".to_string(), base),
			},
		};
		if file.is_empty() {
			bevybail!("`{path}` names no file");
		}
		let uri = match is_uri {
			true => StoreUri::parse(&format!("{dir}{query}"))?,
			false => StoreUri::parse(&format!("fs:{dir}"))?,
		};
		(uri, file.to_string()).xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An identity and the file holding it.
	fn new_identity() -> (AgeIdentity, AgeIdentityFile) {
		let identity = AgeIdentity::generate();
		let mut file = AgeIdentityFile::default();
		file.push(identity.clone());
		(identity, file)
	}

	#[beet_core::test]
	async fn writes_then_reads_any_file() {
		let (identity, identities) = new_identity();
		let store = BlobStore::temp();
		let recipients = [identity.to_recipient()];
		for path in ["secrets.toml.age", "keys/id_ed25519.age"] {
			let vault = VaultHandle::new(store.clone(), path).unwrap();
			vault.exists().await.unwrap().xpect_false();
			vault.write(b"KEY=value\n", &recipients).await.unwrap();
			vault.exists().await.unwrap().xpect_true();
			// armored, so it survives git and paper
			String::from_utf8(
				store.get(&RelPath::new(path)).await.unwrap().to_vec(),
			)
			.unwrap()
			.xpect_starts_with("-----BEGIN AGE ENCRYPTED FILE-----");
			vault
				.read(&identities)
				.await
				.unwrap()
				.xpect_eq(b"KEY=value\n".to_vec());
		}
		VaultHandle::new(store, "notes.toml")
			.unwrap_err()
			.to_string()
			.xpect_contains(".age");
	}

	#[beet_core::test]
	async fn an_unlisted_identity_cannot_open_it() {
		let (identity, _) = new_identity();
		let (_, other) = new_identity();
		let vault = VaultHandle::new(BlobStore::temp(), "a.json.age").unwrap();
		vault
			.write(b"{}", &[identity.to_recipient()])
			.await
			.unwrap();
		vault
			.read(&other)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("other recipients");
	}

	#[beet_core::test]
	fn resolves_a_path_or_uri() {
		let vault =
			VaultHandle::from_uri("memory://vaults/dir/x.toml.age").unwrap();
		vault.path.as_str().xpect_eq("x.toml.age");
		vault.store.subdir().as_str().xpect_eq("dir");
		VaultHandle::from_uri("/tmp/beet/personal.toml.age")
			.unwrap()
			.store
			.base_dir()
			.unwrap()
			.to_string()
			.xpect_eq("/tmp/beet");
		VaultHandle::from_uri("x.json.age")
			.unwrap()
			.store
			.id()
			.xpect_eq("fs");
		VaultHandle::from_uri("memory://x.age").unwrap_err();
		VaultHandle::from_uri("dir/x.txt").unwrap_err();
	}
}
