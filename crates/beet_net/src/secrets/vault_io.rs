//! Vault I/O over any [`BlobStore`].

use crate::prelude::*;
use beet_core::prelude::*;

/// A vault resolved to the store it lives in, the path within it and who may
/// read the next write: what [`VaultQuery`] answers and every vault verb
/// operates on. Reads decrypt with an identity file, writes encrypt to the
/// recipients and land as armored text through the store; the plaintext is
/// bytes in memory, never on disk.
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
/// let vault = VaultHandle::new(BlobStore::temp(), "notes.toml.age")?
/// 	.with_recipients([identity.to_recipient()]);
/// let mut doc = vault.read_or_empty(&identities).await?;
/// doc.set("db.password", "hunter2")?;
/// vault.write(&doc, &vault.write_recipients(&identities)?).await?;
/// vault.read(&identities).await?.xpect_eq(doc);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct VaultHandle {
	/// The declared label, `None` for a vault addressed by path.
	pub label: Option<SmolStr>,
	/// The store the file lives in.
	pub store: BlobStore,
	/// The file within the store.
	pub path: RelPath,
	/// The format the path names.
	pub format: VaultFormat,
	/// Who may read the next write: the declaration's list, or `None` for an
	/// undeclared vault, which is encrypted to the writer's own recipients.
	pub recipients: Option<Vec<AgeRecipient>>,
}

impl VaultHandle {
	/// An undeclared vault at `path` in `store`, encrypted to whoever writes
	/// it until [`with_recipients`](Self::with_recipients) says otherwise.
	pub fn new(store: BlobStore, path: impl AsRef<str>) -> Result<Self> {
		let path = RelPath::new(path);
		Self {
			label: None,
			format: VaultFormat::from_path(path.as_str())?,
			store,
			path,
			recipients: None,
		}
		.xok()
	}

	/// Set the label a verb names it by.
	pub fn with_label(mut self, label: impl Into<SmolStr>) -> Self {
		self.label = Some(label.into());
		self
	}

	/// Declare the recipients.
	pub fn with_recipients(
		mut self,
		recipients: impl IntoIterator<Item = AgeRecipient>,
	) -> Self {
		self.recipients = Some(recipients.into_iter().collect());
		self
	}

	/// How a log or an error names this vault: the label and the path.
	pub fn describe(&self) -> String {
		match &self.label {
			Some(label) => format!("`{label}` ({})", self.path),
			None => format!("`{}`", self.path),
		}
	}

	/// Whether the file exists in its store.
	pub async fn exists(&self) -> Result<bool> {
		self.store.exists(&self.path).await
	}

	/// Read and decrypt the vault with whichever identity in `identities` it
	/// was encrypted to. Errors when the file is missing.
	pub async fn read(
		&self,
		identities: &AgeIdentityFile,
	) -> Result<VaultDocument> {
		let ciphertext = self.store.get(&self.path).await.map_err(|err| {
			bevyhow!("vault {} cannot be read: {err}", self.describe())
		})?;
		VaultDocument::decrypt(self.format, identities, &ciphertext).map_err(
			|err| bevyhow!("vault {} cannot be opened: {err}", self.describe()),
		)
	}

	/// [`read`](Self::read), or an empty document when the file does not
	/// exist yet: the read a first `set` makes.
	pub async fn read_or_empty(
		&self,
		identities: &AgeIdentityFile,
	) -> Result<VaultDocument> {
		match self.exists().await? {
			true => self.read(identities).await,
			false => VaultDocument::new(self.format).xok(),
		}
	}

	/// Encrypt `document` to `recipients` and write it, as armored text.
	pub async fn write(
		&self,
		document: &VaultDocument,
		recipients: &[AgeRecipient],
	) -> Result<()> {
		let ciphertext = document.encrypt(self.format, recipients)?;
		self.store.insert(&self.path, ciphertext).await
	}

	/// Who the next write is encrypted to: the declared list, else the
	/// writer's own recipients (every identity in `identities`), and an error
	/// naming both declarations when a declared list is empty.
	pub fn write_recipients(
		&self,
		identities: &AgeIdentityFile,
	) -> Result<Vec<AgeRecipient>> {
		match &self.recipients {
			Some(recipients) if !recipients.is_empty() => {
				recipients.clone().xok()
			}
			Some(_) => bevybail!(
				"vault {} declares no recipients: name them on the `<Vault \
				recipients={{[..]}}>` or on an ancestor `{{AgeRecipients([..])}}`",
				self.describe()
			),
			None => {
				info!(
					"vault {} is undeclared, so it is encrypted to this \
					identity file's own {} recipient(s)",
					self.describe(),
					identities.len()
				);
				identities.recipients().xok()
			}
		}
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
	async fn writes_then_reads_in_both_kinds() {
		let (identity, identities) = new_identity();
		let store = BlobStore::temp();
		for path in [".env.age", "export.toml.age"] {
			let vault = VaultHandle::new(store.clone(), path)
				.unwrap()
				.with_recipients([identity.to_recipient()]);
			vault.exists().await.unwrap().xpect_false();
			let mut doc = vault.read_or_empty(&identities).await.unwrap();
			doc.set("KEY", "value").unwrap();
			let recipients = vault.write_recipients(&identities).unwrap();
			vault.write(&doc, &recipients).await.unwrap();
			vault.exists().await.unwrap().xpect_true();
			// armored, so it survives git and paper
			String::from_utf8(
				store.get(&RelPath::new(path)).await.unwrap().to_vec(),
			)
			.unwrap()
			.xpect_starts_with("-----BEGIN AGE ENCRYPTED FILE-----");
			vault.read(&identities).await.unwrap().xpect_eq(doc);
		}
	}

	#[beet_core::test]
	async fn an_unlisted_identity_cannot_open_it() {
		let (identity, _) = new_identity();
		let (_, other) = new_identity();
		let vault = VaultHandle::new(BlobStore::temp(), "a.json.age")
			.unwrap()
			.with_recipients([identity.to_recipient()]);
		vault
			.write(&VaultDocument::new(VaultFormat::Json), &[
				identity.to_recipient()
			])
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
	fn write_recipients_resolve_by_declaration() {
		let (identity, identities) = new_identity();
		let store = BlobStore::temp();
		// undeclared: the writer's own
		VaultHandle::new(store.clone(), "a.env.age")
			.unwrap()
			.write_recipients(&identities)
			.unwrap()
			.xpect_eq(vec![identity.to_recipient()]);
		// declared empty: an error naming both declarations
		VaultHandle::new(store, "a.env.age")
			.unwrap()
			.with_recipients([])
			.write_recipients(&identities)
			.unwrap_err()
			.to_string()
			.xpect_contains("AgeRecipients");
	}
}
