//! Age files over any [`BlobStore`].

use crate::prelude::*;
use beet_core::prelude::*;

/// An age file at a path in a store: the file behind `vault/encrypt`,
/// `vault/decrypt` and `vault/rekey`, a key, a certificate, any bytes. A
/// read decrypts with an identity file, a write encrypts plaintext to a
/// recipient list and lands the armored age file through the store; the
/// plaintext is bytes in memory, never on disk. A secret with a name and a
/// role belongs in a secrets document instead.
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
/// let vault = VaultHandle::new(BlobStore::temp(), "cert.pem.age")?;
/// let plaintext = b"-----BEGIN CERTIFICATE-----\n";
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

	/// The file a filesystem path or store uri names, see
	/// [`BlobStore::from_file_uri`]: `~/keys/id_ed25519.age`,
	/// `s3://bucket/certs/x.pem.age`, or a bare `x.age` in the cwd.
	pub fn from_uri(path: &str) -> Result<Self> {
		let (store, file) = BlobStore::from_file_uri(path)?;
		Self::new(store, file)
	}

	/// The one rule a vault path must satisfy: it ends in `.age`, so a write
	/// never lands ciphertext under a plaintext name.
	pub fn validate_path(path: &str) -> Result<()> {
		match path.ends_with(Self::SUFFIX) && path.len() > Self::SUFFIX.len() {
			true => Ok(()),
			false => bevybail!(
				"`{path}` is not a vault: a vault ends in `{}`, ie \
				`cert.pem.age` or `id_ed25519.age`",
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
		for path in ["cert.pem.age", "keys/id_ed25519.age"] {
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
			VaultHandle::from_uri("memory://vaults/dir/x.pem.age").unwrap();
		vault.path.as_str().xpect_eq("x.pem.age");
		vault.store.subdir().as_str().xpect_eq("dir");
		VaultHandle::from_uri("memory://x.age").unwrap_err();
		VaultHandle::from_uri("dir/x.txt").unwrap_err();
	}
}
