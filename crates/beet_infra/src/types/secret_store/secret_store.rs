//! The provider-agnostic seam a stack's tier 2 secrets live behind.

use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt;
use std::sync::Arc;

/// One stack's secret store: parameter store on AWS, a secrets document
/// locally, 1Password or anything else downstream. A handle is scoped to
/// the stack it was resolved for, exactly as a
/// [`BlobStore`](beet_net::prelude::BlobStore) is scoped to a bucket and
/// prefix, so every method takes a [`SecretRef`] label alone and the
/// provider composes its own address from the pair; a label
/// (`dkim-example-com`) never couples to a provider and an export restores
/// into a different provider or region unchanged. Another stack's store is
/// [`for_stack`](Self::for_stack), the `with_subdir` of the seam.
///
/// The erased-provider pattern (`AGENTS.md`): an `Arc<dyn SecretStoreProvider>`
/// cloned by every consumer, landed on the declaring entity by the attach
/// observer of its declaration (`<SsmSecrets/>`, `<DocumentSecrets/>`, each
/// beside its provider) and resolved by [`StackQuery::secret_store`].
/// [`Debug`] prints the provider id and never a value.
///
/// Values never reach a log: a consumer that prints one (`MailCredentials`)
/// says so in its own docs.
///
/// ## Example
///
/// The seam over a document in a temp store, the same calls a deploy makes
/// against parameter store:
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_infra::prelude::*;
/// async_ext::block_on(async {
/// 	let mut identities = AgeIdentityFile::default();
/// 	identities.push(AgeIdentity::generate());
/// 	let stack = Stack::new("app").with_stage("dev").resolve(&default());
/// 	let store = SecretStore::new(
/// 		DocumentSecretStore::new(
/// 			SecretsHandle::new(BlobStore::temp(), "secrets.toml")?,
/// 			stack,
/// 		)?
/// 		.with_identities(identities),
/// 	);
/// 	let secret = SecretRef::new("db-password");
/// 	// create-if-missing: the first call mints, the second reads back
/// 	let (value, minted) = store
/// 		.ensure(&secret, Some("the database"), SecretRotation::Remint, async || {
/// 			Secret::generate("db-password", Secret::GENERATED_LENGTH)
/// 				.map(String::from)
/// 		})
/// 		.await?;
/// 	minted.xpect_true();
/// 	store.get(&secret).await?.xpect_eq(Some(value));
/// 	store.list().await?.len().xpect_eq(1);
/// 	Ok::<_, BevyError>(())
/// })
/// .unwrap();
/// ```
#[derive(Clone, Component)]
pub struct SecretStore {
	provider: Arc<dyn SecretStoreProvider>,
}

impl fmt::Debug for SecretStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SecretStore")
			.field("provider", &self.provider.id())
			.field("stack", &self.provider.stack().resource_name(""))
			.finish()
	}
}

impl SecretStore {
	/// The erased handle over `provider`.
	pub fn new(provider: impl SecretStoreProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// The provider's id, ie `ssm`, `document`.
	pub fn id(&self) -> &'static str { self.provider.id() }

	/// Where the provider lives, for an export's `origin.region`.
	pub fn region(&self) -> Option<SmolStr> { self.provider.region() }

	/// How a log names this store: its id and region.
	pub fn describe(&self) -> String { self.provider.describe() }

	/// The stack this handle is scoped to.
	pub fn stack(&self) -> &ResolvedStack { self.provider.stack() }

	/// The same provider scoped to `stack`: how a drill reads its source
	/// stage's credential through its own store. A provider that cannot
	/// serve another stack (a document holds one) errors naming both.
	pub fn for_stack(&self, stack: &ResolvedStack) -> Result<Self> {
		Self {
			provider: Arc::from(self.provider.for_stack(stack)?),
		}
		.xok()
	}

	/// The provider's address for `secret`, ie `/app/stage/label` on SSM.
	pub fn address(&self, secret: &SecretRef) -> SmolStr {
		self.provider.address(secret)
	}

	/// The value, `None` when the secret does not exist. Any other failure
	/// (no credentials, no identity, no permission) is an error rather than a
	/// silent mint of a second secret.
	pub async fn get(&self, secret: &SecretRef) -> Result<Option<String>> {
		self.provider.get(secret.clone()).await
	}

	/// [`get`](Self::get) for a caller that cannot proceed without the value:
	/// absence is an error naming the address and `hint`, the step that
	/// mints it.
	pub async fn require(
		&self,
		secret: &SecretRef,
		hint: impl FnOnce() -> String,
	) -> Result<String> {
		self.get(secret).await?.ok_or_else(|| {
			bevyhow!("no secret at {}: {}", self.address(secret), hint())
		})
	}

	/// Create `secret`, failing with [`SecretStoreError::AlreadyExists`] when
	/// it exists. Deliberately never an overwrite: two deploys racing to mint
	/// the same secret must not each believe theirs is the one in use, and
	/// the loser re-reads the winner's value instead
	/// ([`ensure`](Self::ensure)). `note` and `rotation` are stored where the
	/// provider can (SSM's description, a document's index); a note is never
	/// a secret, and a rotation is required so no secret arrives
	/// un-rotatable.
	pub async fn create(
		&self,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		rotation: SecretRotation,
	) -> Result {
		self.provider
			.create(secret.clone(), value.into(), SecretMeta {
				note: note.map(SmolStr::new),
				rotation: Some(rotation),
			})
			.await
	}

	/// Create or replace `secret`. The opposite posture to
	/// [`create`](Self::create), for the opposite ownership: a value some
	/// OTHER system mints and this store mirrors (the bootstrap admin
	/// credential a fresh mail server returns exactly once, a pin read off a
	/// box), where an existing value is by definition stale.
	pub async fn overwrite(
		&self,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		rotation: Option<SecretRotation>,
	) -> Result {
		self.provider
			.overwrite(secret.clone(), value.into(), SecretMeta {
				note: note.map(SmolStr::new),
				rotation,
			})
			.await
	}

	/// The stored metadata of one secret, `None` when it does not exist.
	pub async fn meta(&self, secret: &SecretRef) -> Result<Option<SecretMeta>> {
		self.provider.meta(secret.clone()).await
	}

	/// Mirror a value another system derives (a public half, a pin read off
	/// a box): [`overwrite`](Self::overwrite) when the stored value or its
	/// metadata differs from what is declared, nothing when both match.
	/// Answers whether it wrote, so a caller logs the change and not the
	/// steady state; a stale note or rotation converges around an unchanged
	/// value exactly as [`ensure`](Self::ensure) does.
	pub async fn converge(
		&self,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		rotation: Option<SecretRotation>,
	) -> Result<bool> {
		let declared = SecretMeta {
			note: note.map(SmolStr::new),
			rotation: rotation.clone(),
		};
		if self.get(secret).await?.as_deref() == Some(value)
			&& self.meta(secret).await?.as_ref() == Some(&declared)
		{
			return false.xok();
		}
		self.overwrite(secret, value, note, rotation).await?;
		true.xok()
	}

	/// Create-if-missing: the existing value, else `generate` created and
	/// its value, else (the loser of a race) the winner's value re-read.
	/// Answers the value and whether this call minted it. The one shape
	/// every generated credential in a stack takes, so no consumer can
	/// rotate one by accident. An existing value whose stored note or
	/// rotation differs from the declared is rewritten unchanged with the
	/// declared metadata, since the mint site is where both are declared
	/// and the store only mirrors them.
	pub async fn ensure(
		&self,
		secret: &SecretRef,
		note: Option<&str>,
		rotation: SecretRotation,
		generate: impl AsyncFnOnce() -> Result<String>,
	) -> Result<(String, bool)> {
		let address = self.address(secret);
		if let Some(value) = self.get(secret).await? {
			let declared = SecretMeta {
				note: note.map(SmolStr::new),
				rotation: Some(rotation),
			};
			if self.meta(secret).await?.as_ref() != Some(&declared) {
				self.overwrite(secret, &value, note, declared.rotation)
					.await?;
				info!("converged the metadata of secret {address}");
			}
			return (value, false).xok();
		}
		let generated = generate().await?;
		match self.create(secret, &generated, note, rotation).await {
			Ok(()) => (generated, true).xok(),
			Err(err) if SecretStoreError::is_already_exists(&err) => {
				info!("secret {address} was minted concurrently, re-reading");
				self.get(secret)
					.await?
					.map(|value| (value, false))
					.ok_or_else(|| {
						bevyhow!("secret {address} vanished between writes")
					})
			}
			Err(err) => Err(err),
		}
	}

	/// Every secret of the stack, with its metadata and never a value.
	pub async fn list(&self) -> Result<Vec<SecretEntry>> {
		self.provider.list().await
	}

	/// Every secret of the stack with its value: what an export reads.
	pub async fn read_all(&self) -> Result<Vec<(SecretEntry, String)>> {
		self.provider.read_all().await
	}

	/// Delete `secrets`, answering the ones actually deleted; one already
	/// gone is not an error.
	pub async fn delete(
		&self,
		secrets: &[SecretRef],
	) -> Result<Vec<SecretRef>> {
		self.provider.delete(secrets.to_vec()).await
	}
}

/// A secret store backend scoped to one stack, see [`SecretStore`].
pub trait SecretStoreProvider: 'static + Send + Sync {
	/// A boxed clone, so a default method can own the provider across an
	/// await.
	fn box_clone(&self) -> Box<dyn SecretStoreProvider>;

	/// Stable provider discriminator, ie `ssm`, `document`.
	fn id(&self) -> &'static str;

	/// The provider's region, where it has one.
	fn region(&self) -> Option<SmolStr>;

	/// Where the store lives, for a log: the id and region by default.
	fn describe(&self) -> String {
		match self.region() {
			Some(region) => format!("{} ({region})", self.id()),
			None => self.id().to_string(),
		}
	}

	/// The stack this provider is scoped to.
	fn stack(&self) -> &ResolvedStack;

	/// See [`SecretStore::for_stack`].
	fn for_stack(
		&self,
		stack: &ResolvedStack,
	) -> Result<Box<dyn SecretStoreProvider>>;

	/// The provider's address for `secret`.
	fn address(&self, secret: &SecretRef) -> SmolStr;

	/// See [`SecretStore::get`].
	fn get(&self, secret: SecretRef)
	-> SendBoxedFuture<Result<Option<String>>>;

	/// See [`SecretStore::create`].
	fn create(
		&self,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result>;

	/// See [`SecretStore::overwrite`].
	fn overwrite(
		&self,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result>;

	/// See [`SecretStore::list`].
	fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>>;

	/// See [`SecretStore::meta`]: found in the listing, which a provider
	/// with a cheaper single read overrides.
	fn meta(
		&self,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<SecretMeta>>> {
		let this = self.box_clone();
		Box::pin(async move {
			this.list()
				.await?
				.into_iter()
				.find(|entry| entry.secret.label() == secret.label())
				.map(|entry| SecretMeta {
					note: entry.note,
					rotation: entry.rotation,
				})
				.xok()
		})
	}

	/// See [`SecretStore::read_all`]: a listing then a read per entry, which
	/// a provider with a bulk read overrides.
	fn read_all(&self) -> SendBoxedFuture<Result<Vec<(SecretEntry, String)>>> {
		let this = self.box_clone();
		Box::pin(async move {
			let mut values = Vec::new();
			for entry in this.list().await? {
				let value =
					this.get(entry.secret.clone()).await?.ok_or_else(|| {
						bevyhow!(
							"secret {} vanished between the listing and the read",
							entry.address
						)
					})?;
				values.push((entry, value));
			}
			values.xok()
		})
	}

	/// See [`SecretStore::delete`].
	fn delete(
		&self,
		secrets: Vec<SecretRef>,
	) -> SendBoxedFuture<Result<Vec<SecretRef>>>;
}

/// What a store knows about one secret without handing over its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretEntry {
	/// The secret's label, the name a restore composes a fresh address from.
	pub secret: SecretRef,
	/// The provider's address at the time of the listing.
	pub address: SmolStr,
	/// The note stored at mint, where the provider keeps one.
	pub note: Option<SmolStr>,
	/// When the value was last written, where the provider says.
	pub modified: Option<Timestamp>,
	/// How the secret rotates, stored at mint; absent on one parked by hand.
	pub rotation: Option<SecretRotation>,
}

/// What a write stores beside a value: the note and the rotation, which a
/// provider keeps where it can (SSM's description, a document's index).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SecretMeta {
	/// A plaintext note, never a secret: what the value is for.
	pub note: Option<SmolStr>,
	/// How the value rotates; every mint declares one, a restore carries
	/// the record's.
	pub rotation: Option<SecretRotation>,
}

/// The one failure a consumer matches on: [`SecretStore::create`] found the
/// secret already there, so the caller re-reads rather than overwrites.
#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
	#[error("secret {address} already exists")]
	AlreadyExists { address: SmolStr },
}

impl SecretStoreError {
	/// Whether `err` is the create conflict, ie somebody else won the race.
	pub fn is_already_exists(err: &BevyError) -> bool {
		matches!(err.downcast_ref::<Self>(), Some(Self::AlreadyExists { .. }))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::types::test_support::*;
	use beet_net::prelude::*;

	/// A stack root carrying the repo store, with `declarations` under it.
	fn stack_with(world: &mut World, declarations: impl Bundle) -> Entity {
		let root = world
			.spawn((
				Stack::new("app").with_stage("prod"),
				AwsRegion::new(Stack::TEST_AWS_REGION),
				BlobStore::temp(),
				RepoStore,
				children![declarations],
			))
			.id();
		world.flush();
		root
	}

	/// The declared store wins, resolved from anywhere under the stack and
	/// scoped to it.
	#[beet_core::test]
	fn resolution_picks_the_declared_store() {
		let mut world = infra_world();
		let root = stack_with(
			&mut world,
			DocumentSecrets::new("infra/secrets/app--prod.toml"),
		);
		let consumer = world.spawn(ChildOf(root)).id();
		world.flush();
		world.with_state::<StackQuery, _>(|stacks| {
			let store = stacks.secret_store(consumer).unwrap();
			store.id().xpect_eq("document");
			store
				.describe()
				.xpect_contains("infra/secrets/app--prod.toml");
			store.stack().clone().xpect_eq(stacks.resolve(root));
		});
	}

	/// With nothing declared the stack carries an implicit `<SsmSecrets/>`,
	/// parameter store in the stack's region whatever the launch, and a
	/// declared one is the same store.
	#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn an_undeclared_stack_is_ssm() {
		let mut world = infra_world();
		let root = stack_with(&mut world, ());
		world.with_state::<StackQuery, _>(|stacks| {
			let stack = stacks.resolve(root);
			let store = stacks.secret_store(root).unwrap();
			store.id().xpect_eq("ssm");
			store
				.region()
				.unwrap()
				.xpect_eq(stack.aws_region().unwrap().clone());
			store.stack().clone().xpect_eq(stack);
		});
		// (one repo store per world, so a second world)
		let mut world = infra_world();
		let declared = stack_with(&mut world, SsmSecrets);
		world.with_state::<StackQuery, _>(|stacks| {
			stacks.secret_store(declared).unwrap().id().xpect_eq("ssm");
		});
	}

	/// Two declarations under one stack is an error naming both.
	#[beet_core::test]
	fn two_declarations_clobber() {
		let mut world = infra_world();
		let root = stack_with(
			&mut world,
			(DocumentSecrets::new("a.toml"), children![
				DocumentSecrets::new("b.toml")
			]),
		);
		world.with_state::<StackQuery, _>(|stacks| {
			stacks
				.secret_store(root)
				.unwrap_err()
				.to_string()
				.xpect_contains("two secret stores")
				.xpect_contains("a.toml")
				.xpect_contains("b.toml");
		});
	}

	/// The loser of a mint race re-reads the winner's value rather than
	/// overwriting it: a store whose first read misses and whose create then
	/// conflicts.
	#[beet_core::test]
	async fn ensure_re_reads_after_losing_a_race() {
		#[derive(Clone)]
		struct Racing {
			inner: DocumentSecretStore,
			misses: Arc<core::sync::atomic::AtomicUsize>,
		}
		impl SecretStoreProvider for Racing {
			fn box_clone(&self) -> Box<dyn SecretStoreProvider> {
				Box::new(self.clone())
			}
			fn id(&self) -> &'static str { "racing" }
			fn region(&self) -> Option<SmolStr> { None }
			fn stack(&self) -> &ResolvedStack { self.inner.stack() }
			fn for_stack(
				&self,
				stack: &ResolvedStack,
			) -> Result<Box<dyn SecretStoreProvider>> {
				self.inner.for_stack(stack)
			}
			fn address(&self, secret: &SecretRef) -> SmolStr {
				self.inner.address(secret)
			}
			/// The first read misses, as the loser's did.
			fn get(
				&self,
				secret: SecretRef,
			) -> SendBoxedFuture<Result<Option<String>>> {
				use core::sync::atomic::Ordering;
				match self.misses.fetch_sub(1, Ordering::Relaxed) > 0 {
					true => Box::pin(async { None.xok() }),
					false => self.inner.get(secret),
				}
			}
			fn create(
				&self,
				secret: SecretRef,
				value: SmolStr,
				meta: SecretMeta,
			) -> SendBoxedFuture<Result> {
				self.inner.create(secret, value, meta)
			}
			fn overwrite(
				&self,
				secret: SecretRef,
				value: SmolStr,
				meta: SecretMeta,
			) -> SendBoxedFuture<Result> {
				self.inner.overwrite(secret, value, meta)
			}
			fn list(&self) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
				self.inner.list()
			}
			fn delete(
				&self,
				secrets: Vec<SecretRef>,
			) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
				self.inner.delete(secrets)
			}
		}
		let stack = Stack::new("app").resolve(&PackageConfig::default());
		let inner = memory_store(&stack);
		let secret = SecretRef::new("db-password");
		// the winner minted first
		SecretStore::new(inner.clone())
			.create(&secret, "winner", None, SecretRotation::Remint)
			.await
			.unwrap();
		let store = SecretStore::new(Racing {
			inner,
			misses: Arc::new(1.into()),
		});
		let (value, minted) = store
			.ensure(&secret, None, SecretRotation::Remint, async || {
				"loser".to_string().xok()
			})
			.await
			.unwrap();
		value.as_str().xpect_eq("winner");
		minted.xpect_false();
	}

	#[beet_core::test]
	fn already_exists_is_matched_by_type() {
		let err: BevyError = SecretStoreError::AlreadyExists {
			address: "/app/dev/x".into(),
		}
		.into();
		SecretStoreError::is_already_exists(&err).xpect_true();
		SecretStoreError::is_already_exists(&bevyhow!(
			"secret /app/dev/x already exists"
		))
		.xpect_false();
	}
}
