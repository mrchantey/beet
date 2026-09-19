//! A secrets document as a stack's secret store.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A [`SecretStore`] over one secrets document: every secret a record in
/// the document's `default` group with its `note` and `modified`, the label
/// its record name. The local stand-in for parameter store (a `Local` launch
/// resolves one over `target/secrets/<app>--<stage>.toml`, decision: the
/// same naming as `target/stores/<app>--<stage>--<label>`) and the
/// human-side store a `<DocumentSecrets path=".."/>` declares in the repo.
///
/// One document holds one stack's secrets, so [`for_stack`] to any other
/// stack refuses by name rather than reading the wrong file. The identity is
/// the discovered one, allowed to be absent at construction so a `validate`
/// with no identity still resolves a store: a read of a sealed group or any
/// write then fails with the `keygen` guidance.
///
/// [`for_stack`]: SecretStoreProvider::for_stack
///
/// The `default` group is seeded on the first write from `seed`'s own
/// `default` list when a seed document is given (the entry's declared
/// document, so the humans are listed once) plus this identity, so the
/// writer can always read back what it wrote; with no seed, from the
/// identity file's own recipients.
#[derive(Debug, Clone)]
pub struct DocumentSecretStore {
	handle: SecretsHandle,
	stack: ResolvedStack,
	identities: AgeIdentityFile,
	seed: Option<SecretsHandle>,
}

impl DocumentSecretStore {
	/// The provider id.
	pub const ID: &'static str = "document";

	/// Where a `Local` launch keeps a stack's stand-in document.
	pub const LOCAL_DIR: &'static str = "target/secrets";

	/// The store over `handle` for `stack`, opened with the discovered
	/// identity (an empty file when there is none).
	pub fn new(handle: SecretsHandle, stack: ResolvedStack) -> Result<Self> {
		Self {
			handle,
			stack,
			identities: AgeIdentityFile::discover()?.unwrap_or_default(),
			seed: None,
		}
		.xok()
	}

	/// The store with an explicit identity file, for a test.
	pub fn with_identities(mut self, identities: AgeIdentityFile) -> Self {
		self.identities = identities;
		self
	}

	/// Seed a new document's `default` group from `seed`'s.
	pub fn with_seed(mut self, seed: Option<SecretsHandle>) -> Self {
		self.seed = seed;
		self
	}

	/// The `Local` stand-in for `stack`: `target/secrets/<app>--<stage>.toml`
	/// in the workspace.
	pub fn local(stack: ResolvedStack) -> Result<Self> {
		let dir = WsPath::new(Self::LOCAL_DIR).into_abs();
		let path = format!(
			"{}--{}.{}",
			stack.app_name(),
			stack.stage(),
			SecretsDocument::default_media_type()
				.extension()
				.unwrap_or_default()
		);
		Self::new(
			SecretsHandle::new(BlobStore::new(FsStore::new(dir)), path)?,
			stack,
		)
	}

	/// The document this store reads and writes.
	pub fn handle(&self) -> &SecretsHandle { &self.handle }

	/// The document, or a new one seeded from the seed's `default` group.
	async fn read_or_seed(&self) -> Result<SecretsDocument> {
		if self.handle.exists().await? {
			return self.handle.read().await;
		}
		let mut document = SecretsDocument::new(self.handle.media_type()?);
		if let Some(seed) = &self.seed
			&& seed.exists().await?
			&& let Some(group) =
				seed.read().await?.groups.get(SecretRecord::DEFAULT_GROUP)
		{
			let mut recipients = group.recipients.clone();
			for own in self.identities.recipients() {
				if !recipients.contains(&own) {
					recipients.push(own);
				}
			}
			document.groups.insert(
				SecretRecord::DEFAULT_GROUP.into(),
				SecretsGroup::new(recipients),
			);
		}
		document.xok()
	}

	/// Open the document's `default` group, the one every record here lives
	/// in; an identity that cannot is an error with the `keygen` guidance.
	async fn open(&self, document: &SecretsDocument) -> Result<OpenSecrets> {
		let opened = document.open(&self.identities)?;
		if !document.groups.is_empty()
			&& !opened.can_open(SecretRecord::DEFAULT_GROUP)
		{
			bevybail!(
				"this identity cannot open group `{}` of document {}{}",
				SecretRecord::DEFAULT_GROUP,
				self.handle.describe(),
				match self.identities.is_empty() {
					true => format!(
						": no age identity (`beet vault/keygen` makes one, \
						`vault/restore-identity` restores one, a runner passes \
						one through `{}`)",
						AgeIdentityFile::ENV_VAR
					),
					false => String::new(),
				}
			);
		}
		opened.xok()
	}

	async fn write(
		&self,
		secret: &SecretRef,
		value: &str,
		meta: SecretMeta,
		create_only: bool,
	) -> Result {
		let mut document = self.read_or_seed().await?;
		if create_only && document.secrets.contains_key(secret.label()) {
			return Err(SecretStoreError::AlreadyExists {
				address: self.address(secret),
			}
			.into());
		}
		document.set(
			&self.identities,
			secret.label(),
			value,
			SecretRecord {
				note: meta.note,
				rotation: meta.rotation,
				..default()
			},
		)?;
		self.handle.write(&document).await
	}

	fn entry(&self, name: &SmolStr, record: &SecretRecord) -> SecretEntry {
		let secret = SecretRef::new(name.clone());
		SecretEntry {
			address: self.address(&secret),
			secret,
			note: record.note.clone(),
			modified: record.modified,
			rotation: record.rotation.clone(),
		}
	}
}

impl SecretStoreProvider for DocumentSecretStore {
	fn box_clone(&self) -> Box<dyn SecretStoreProvider> {
		Box::new(self.clone())
	}

	fn id(&self) -> &'static str { Self::ID }

	fn region(&self) -> Option<SmolStr> { None }

	fn describe(&self) -> String {
		format!("{} {}", Self::ID, self.handle.describe())
	}

	fn stack(&self) -> &ResolvedStack { &self.stack }

	/// The one document holds one stack: another is an error naming both.
	fn for_stack(
		&self,
		stack: &ResolvedStack,
	) -> Result<Box<dyn SecretStoreProvider>> {
		match stack == &self.stack {
			true => Ok(self.box_clone()),
			false => bevybail!(
				"document {} holds the secrets of `{}--{}`, not `{}--{}`: a \
				document store serves one stack",
				self.handle.describe(),
				self.stack.app_name(),
				self.stack.stage(),
				stack.app_name(),
				stack.stage()
			),
		}
	}

	/// The document's path and the record's name, ie
	/// `target/secrets/app--dev.toml#dkim-example-com`.
	fn address(&self, secret: &SecretRef) -> SmolStr {
		format!("{}#{}", self.handle.path, secret.label()).into()
	}

	fn get(
		&self,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<String>>> {
		let this = self.clone();
		Box::pin(async move {
			if !this.handle.exists().await? {
				return None.xok();
			}
			let document = this.handle.read().await?;
			if !document.secrets.contains_key(secret.label()) {
				return None.xok();
			}
			this.open(&document)
				.await?
				.get(secret.label())
				.map(|secret| secret.value.to_string())
				.xok()
		})
	}

	fn create(
		&self,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move { this.write(&secret, &value, meta, true).await })
	}

	fn overwrite(
		&self,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result> {
		let this = self.clone();
		Box::pin(async move { this.write(&secret, &value, meta, false).await })
	}

	/// The index, which needs no identity.
	fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
		let this = self.clone();
		Box::pin(async move {
			if !this.handle.exists().await? {
				return Vec::new().xok();
			}
			this.handle
				.read()
				.await?
				.secrets
				.iter()
				.map(|(name, record)| this.entry(name, record))
				.collect::<Vec<_>>()
				.xok()
		})
	}

	/// One open of the document rather than one per record.
	fn read_all(&self) -> SendBoxedFuture<Result<Vec<(SecretEntry, String)>>> {
		let this = self.clone();
		Box::pin(async move {
			if !this.handle.exists().await? {
				return Vec::new().xok();
			}
			let document = this.handle.read().await?;
			let opened = this.open(&document).await?;
			document
				.secrets
				.iter()
				.map(|(name, record)| {
					let value = opened
						.get(name)
						.map(|secret| secret.value.to_string())
						.ok_or_else(|| {
							bevyhow!(
								"record `{name}` is in the index of {} but not \
								in a group this identity opens",
								this.handle.describe()
							)
						})?;
					(this.entry(name, record), value).xok()
				})
				.collect()
		})
	}

	fn delete(
		&self,
		secrets: Vec<SecretRef>,
	) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
		let this = self.clone();
		Box::pin(async move {
			if !this.handle.exists().await? {
				return Vec::new().xok();
			}
			let mut document = this.handle.read().await?;
			let mut deleted = Vec::new();
			for secret in secrets {
				if !document.secrets.contains_key(secret.label()) {
					continue;
				}
				document.remove(&this.identities, secret.label())?;
				deleted.push(secret);
			}
			if !deleted.is_empty() {
				this.handle.write(&document).await?;
			}
			deleted.xok()
		})
	}
}

/// Observer: land the [`SecretStore`] a `<DocumentSecrets/>` declares on
/// its entity, the document at its path in the nearest ancestor store (the
/// repo store), bound to the stack it is declared under and seeded from the
/// entry's declared document. Deferred through the command queue because
/// the stack and the store are ancestors, which land after insertion.
pub(crate) fn attach_document_secrets(
	ev: On<Insert, DocumentSecrets>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let path = entity.get_or_else::<DocumentSecrets>()?.path.clone();
			if !entity.world().contains_resource::<PackageConfig>() {
				bevybail!(
					"resolving the secrets document `{path}` needs the \
					`PackageConfig` resource, which `BootstrapPlugin` inserts"
				);
			}
			let (stack, store, seed) = entity
				.with_state::<(StackQuery, SecretsQuery), _>(
					|entity, (stacks, documents)| {
						(
							stacks.resolve(entity),
							documents.entry_store(entity),
							documents.resolve_default(entity).ok(),
						)
					},
				);
			let handle = SecretsHandle::new(store?, path.as_str())?;
			entity.insert(SecretStore::new(
				DocumentSecretStore::new(handle, stack)?.with_seed(seed),
			));
			Ok(())
		});
}

#[cfg(test)]
pub(crate) mod test_support {
	use super::*;
	use std::sync::LazyLock;

	/// The one identity every seam test runs as, set inline through
	/// `BEET_AGE_IDENTITY` exactly once per process (one human per machine),
	/// so a store built through discovery finds it: a per-test value would
	/// race the shared environment.
	static IDENTITY: LazyLock<AgeIdentity> = LazyLock::new(|| {
		let identity = AgeIdentity::generate();
		// SAFETY: test-only, set once before any seam test reads it
		unsafe {
			env_ext::set_var(AgeIdentityFile::ENV_VAR, &identity.to_string())
				.unwrap();
		}
		identity
	});

	/// The process identity, forced into the environment.
	pub(crate) fn process_identity() -> AgeIdentity { IDENTITY.clone() }

	/// A document store over a memory store for `stack`, as the process
	/// identity: the store every seam test runs against.
	pub(crate) fn memory_store(stack: &ResolvedStack) -> DocumentSecretStore {
		let mut identities = AgeIdentityFile::default();
		identities.push(process_identity());
		DocumentSecretStore::new(
			SecretsHandle::new(BlobStore::temp(), "secrets.toml").unwrap(),
			stack.clone(),
		)
		.unwrap()
		.with_identities(identities)
	}

	/// A world a stack's secret store resolves in: the infra plugin and the
	/// process identity every declaration composes from.
	pub(crate) fn infra_world() -> World {
		process_identity();
		let mut world =
			(AsyncPlugin, BootstrapPlugin, InfraPlugin).into_world();
		world.init_resource::<PackageConfig>();
		world
	}

	/// The erased store over [`memory_store`].
	pub(crate) fn memory_secret_store(stack: &ResolvedStack) -> SecretStore {
		SecretStore::new(memory_store(stack))
	}
}

#[cfg(test)]
mod test {
	use super::test_support::*;
	use super::*;

	fn stack() -> ResolvedStack {
		Stack::new("beet_infra")
			.with_stage("dev")
			.resolve(&PackageConfig::default())
	}

	/// The whole seam against a document: create refuses a duplicate,
	/// overwrite replaces, the note and modified read back in the listing,
	/// and delete answers what it removed.
	#[beet_core::test]
	async fn implements_the_seam() {
		let stack = stack();
		let store = memory_secret_store(&stack);
		let secret = SecretRef::new("db-password");
		store.get(&secret).await.unwrap().xpect_none();
		store.list().await.unwrap().len().xpect_eq(0);
		store
			.create(
				&secret,
				"hunter2",
				Some("the database"),
				SecretRotation::Remint,
			)
			.await
			.unwrap();
		store
			.get(&secret)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq("hunter2");
		let err = store
			.create(&secret, "other", None, SecretRotation::Remint)
			.await
			.unwrap_err();
		SecretStoreError::is_already_exists(&err).xpect_true();
		store
			.overwrite(
				&secret,
				"hunter3",
				Some("rotated"),
				Some(SecretRotation::manual("by hand")),
			)
			.await
			.unwrap();
		let entries = store.list().await.unwrap();
		entries.len().xpect_eq(1);
		entries[0].secret.label().as_str().xpect_eq("db-password");
		entries[0]
			.address
			.as_str()
			.xpect_eq("secrets.toml#db-password");
		entries[0]
			.note
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("rotated");
		entries[0].modified.xpect_some();
		entries[0]
			.rotation
			.clone()
			.xpect_eq(Some(SecretRotation::manual("by hand")));
		let all = store.read_all().await.unwrap();
		all[0].1.as_str().xpect_eq("hunter3");
		store
			.delete(&[secret.clone(), SecretRef::new("nope")])
			.await
			.unwrap()
			.xpect_eq(vec![secret.clone()]);
		store.get(&secret).await.unwrap().xpect_none();
	}

	/// The race path: the loser re-reads the winner's value.
	#[beet_core::test]
	async fn ensure_is_create_if_missing() {
		let stack = stack();
		let store = memory_secret_store(&stack);
		let secret = SecretRef::new("db-password");
		let (value, minted) = store
			.ensure(&secret, Some("db"), SecretRotation::Remint, async || {
				"first".to_string().xok()
			})
			.await
			.unwrap();
		value.as_str().xpect_eq("first");
		minted.xpect_true();
		let (value, minted) = store
			.ensure(&secret, None, SecretRotation::Remint, async || {
				"second".to_string().xok()
			})
			.await
			.unwrap();
		value.as_str().xpect_eq("first");
		minted.xpect_false();
	}

	/// One document, one stack: rescoping to another stage is refused by
	/// name, and to the same stage is the same store.
	#[beet_core::test]
	fn serves_one_stack() {
		let stack = stack();
		let store = memory_secret_store(&stack);
		let other = Stack::new("beet_infra")
			.with_stage("prod")
			.resolve(&PackageConfig::default());
		store
			.for_stack(&other)
			.unwrap_err()
			.to_string()
			.xpect_contains("`beet_infra--dev`")
			.xpect_contains("`beet_infra--prod`");
		store
			.for_stack(&stack)
			.unwrap()
			.stack()
			.clone()
			.xpect_eq(stack);
	}

	/// A store with no identity lists the index and refuses to open a
	/// value, naming `keygen`.
	#[beet_core::test]
	async fn no_identity_reads_the_index_only() {
		let stack = stack();
		let seeded = memory_store(&stack);
		let secret = SecretRef::new("x");
		let store = SecretStore::new(seeded.clone());
		store
			.create(&secret, "1", None, SecretRotation::Remint)
			.await
			.unwrap();
		let blind = SecretStore::new(
			seeded.with_identities(AgeIdentityFile::default()),
		);
		blind.list().await.unwrap().len().xpect_eq(1);
		blind
			.get(&secret)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no age identity");
		blind
			.get(&SecretRef::new("absent"))
			.await
			.unwrap()
			.xpect_none();
	}

	/// A new document's `default` group copies the seed's list, plus the
	/// writer so it reads back what it wrote.
	#[beet_core::test]
	async fn seeds_default_from_the_declared_document() {
		let stack = stack();
		let alice = AgeIdentity::generate();
		let mut alice_file = AgeIdentityFile::default();
		alice_file.push(alice.clone());
		let seed =
			SecretsHandle::new(BlobStore::temp(), "secrets.toml").unwrap();
		let mut document = SecretsDocument::default();
		document.set(&alice_file, "A", "1", default()).unwrap();
		seed.write(&document).await.unwrap();
		let store = memory_store(&stack);
		let store_identities = store.identities.clone();
		let store = store.with_seed(Some(seed));
		let bound = SecretStore::new(store.clone());
		bound
			.create(&SecretRef::new("x"), "1", None, SecretRotation::Remint)
			.await
			.unwrap();
		let written = store.handle().read().await.unwrap();
		let recipients = &written.groups["default"].recipients;
		recipients.contains(&alice.to_recipient()).xpect_true();
		recipients.len().xpect_eq(2);
		// both alice and the writer open it
		written.open(&alice_file).unwrap().get("x").xpect_some();
		written
			.open(&store_identities)
			.unwrap()
			.get("x")
			.xpect_some();
	}
}
