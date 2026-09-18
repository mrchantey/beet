//! The provider-agnostic seam a stack's tier 2 secrets live behind.

use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt;
use std::sync::Arc;

/// A stack's secret store: parameter store on AWS, a secrets document
/// locally, 1Password or anything else downstream. Every secret is addressed
/// by `(stack, SecretRef)` and every provider composes its own address from
/// the pair, so a label (`dkim-example-com`) never couples to a provider and
/// an export restores into a different provider or region unchanged.
///
/// Erased like [`BlobStore`]: an `Arc<dyn SecretStoreProvider>`, cloned by
/// every consumer, landed on the declaring entity by the attach observer of
/// its declaration (`<SsmSecrets/>`, `<DocumentSecrets/>`) and resolved by
/// [`StackQuery::secret_store`]. [`Debug`] prints the provider id and never
/// a value.
///
/// Values never reach a log: a consumer that prints one (`MailCredentials`)
/// says so in its own docs.
#[derive(Clone, Component)]
pub struct SecretStore {
	provider: Arc<dyn SecretStoreProvider>,
}

impl fmt::Debug for SecretStore {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SecretStore")
			.field("provider", &self.provider.id())
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

	/// The provider's address for `secret` in `stack`, ie
	/// `/app/stage/label` on SSM.
	pub fn address(
		&self,
		stack: &ResolvedStack,
		secret: &SecretRef,
	) -> SmolStr {
		self.provider.address(stack, secret)
	}

	/// The value, `None` when the secret does not exist. Any other failure
	/// (no credentials, no identity, no permission) is an error rather than a
	/// silent mint of a second secret.
	pub async fn get(
		&self,
		stack: &ResolvedStack,
		secret: &SecretRef,
	) -> Result<Option<String>> {
		self.provider.get(stack.clone(), secret.clone()).await
	}

	/// [`get`](Self::get) for a caller that cannot proceed without the value:
	/// absence is an error naming the address and `hint`, the step that
	/// mints it.
	pub async fn require(
		&self,
		stack: &ResolvedStack,
		secret: &SecretRef,
		hint: impl FnOnce() -> String,
	) -> Result<String> {
		self.get(stack, secret).await?.ok_or_else(|| {
			bevyhow!("no secret at {}: {}", self.address(stack, secret), hint())
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
		stack: &ResolvedStack,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		rotation: Rotation,
	) -> Result {
		self.provider
			.create(stack.clone(), secret.clone(), value.into(), SecretMeta {
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
		stack: &ResolvedStack,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		rotation: Option<Rotation>,
	) -> Result {
		self.provider
			.overwrite(
				stack.clone(),
				secret.clone(),
				value.into(),
				SecretMeta {
					note: note.map(SmolStr::new),
					rotation,
				},
			)
			.await
	}

	/// Create-if-missing: the existing value, else `generate` created and
	/// its value, else (the loser of a race) the winner's value re-read.
	/// Answers the value and whether this call minted it. The one shape
	/// every generated credential in a stack takes, so no consumer can
	/// rotate one by accident.
	pub async fn ensure(
		&self,
		stack: &ResolvedStack,
		secret: &SecretRef,
		note: Option<&str>,
		rotation: Rotation,
		generate: impl AsyncFnOnce() -> Result<String>,
	) -> Result<(String, bool)> {
		let address = self.address(stack, secret);
		if let Some(value) = self.get(stack, secret).await? {
			return (value, false).xok();
		}
		let generated = generate().await?;
		match self.create(stack, secret, &generated, note, rotation).await {
			Ok(()) => (generated, true).xok(),
			Err(err) if SecretStoreError::is_already_exists(&err) => {
				info!("secret {address} was minted concurrently, re-reading");
				self.get(stack, secret)
					.await?
					.map(|value| (value, false))
					.ok_or_else(|| {
						bevyhow!("secret {address} vanished between writes")
					})
			}
			Err(err) => Err(err),
		}
	}

	/// Every secret of `stack`, with its metadata and never a value.
	pub async fn list(
		&self,
		stack: &ResolvedStack,
	) -> Result<Vec<SecretEntry>> {
		self.provider.list(stack.clone()).await
	}

	/// Every secret of `stack` with its value: what an export reads.
	pub async fn read_all(
		&self,
		stack: &ResolvedStack,
	) -> Result<Vec<(SecretEntry, String)>> {
		self.provider.read_all(stack.clone()).await
	}

	/// Delete `secrets`, answering the ones actually deleted; one already
	/// gone is not an error.
	pub async fn delete(
		&self,
		stack: &ResolvedStack,
		secrets: &[SecretRef],
	) -> Result<Vec<SecretRef>> {
		self.provider.delete(stack.clone(), secrets.to_vec()).await
	}
}

/// A secret store backend, see [`SecretStore`]. Every method takes the stack
/// the secret belongs to, since one provider (a region's parameter store)
/// serves many stacks and a drill reads its source stage's.
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

	/// The provider's address for `secret` in `stack`.
	fn address(&self, stack: &ResolvedStack, secret: &SecretRef) -> SmolStr;

	/// See [`SecretStore::get`].
	fn get(
		&self,
		stack: ResolvedStack,
		secret: SecretRef,
	) -> SendBoxedFuture<Result<Option<String>>>;

	/// See [`SecretStore::create`].
	fn create(
		&self,
		stack: ResolvedStack,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result>;

	/// See [`SecretStore::overwrite`].
	fn overwrite(
		&self,
		stack: ResolvedStack,
		secret: SecretRef,
		value: SmolStr,
		meta: SecretMeta,
	) -> SendBoxedFuture<Result>;

	/// See [`SecretStore::list`].
	fn list(
		&self,
		stack: ResolvedStack,
	) -> SendBoxedFuture<Result<Vec<SecretEntry>>>;

	/// See [`SecretStore::read_all`]: a listing then a read per entry, which
	/// a provider with a bulk read overrides.
	fn read_all(
		&self,
		stack: ResolvedStack,
	) -> SendBoxedFuture<Result<Vec<(SecretEntry, String)>>> {
		let this = self.box_clone();
		Box::pin(async move {
			let mut values = Vec::new();
			for entry in this.list(stack.clone()).await? {
				let value = this
					.get(stack.clone(), entry.secret.clone())
					.await?
					.ok_or_else(|| {
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
		stack: ResolvedStack,
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
	pub rotation: Option<Rotation>,
}

/// What a write stores beside a value: the note and the rotation, which a
/// provider keeps where it can (SSM's description, a document's index).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SecretMeta {
	/// A plaintext note, never a secret: what the value is for.
	pub note: Option<SmolStr>,
	/// How the value rotates; every mint declares one, a restore carries
	/// the record's.
	pub rotation: Option<Rotation>,
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

/// Declares that the stack's secrets live in AWS parameter store, in the
/// stack's region: `<SsmSecrets/>` under a `<Stack>`. The default for a
/// [`Remote`](ServiceAccess::Remote) launch, so a stack on AWS need not
/// declare it. Native and `deploy` only, since the provider drives the `aws`
/// cli.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct SsmSecrets;

/// Declares that the stack's secrets live in a secrets document:
/// `<DocumentSecrets path="infra/secrets/app--prod.toml"/>` under a
/// `<Stack>`, the file in the nearest ancestor `BlobStore` (the repo store).
/// The default for a [`Local`](ServiceAccess::Local) launch is the same store
/// over `target/secrets/<app>--<stage>.toml`, so a stack need not declare it
/// to run locally.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct DocumentSecrets {
	/// The document within its store, its format named by its extension.
	pub path: RelPath,
}

impl Default for DocumentSecrets {
	fn default() -> Self {
		Self {
			path: RelPath::new(SecretsDocument::DEFAULT_PATH),
		}
	}
}

impl DocumentSecrets {
	/// The declaration for the document at `path`.
	pub fn new(path: impl AsRef<str>) -> Self {
		Self {
			path: RelPath::new(path),
		}
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
				BlobStore::temp(),
				RepoStore,
				children![declarations],
			))
			.id();
		world.flush();
		root
	}

	/// The declared store wins, resolved from anywhere under the stack.
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
		});
	}

	/// With nothing declared the launch's access decides: local is the
	/// stand-in document under `target/secrets`, remote is parameter store
	/// in the stack's region.
	#[beet_core::test]
	fn resolution_falls_back_by_service_access() {
		let mut world = infra_world();
		let root = stack_with(&mut world, ());
		world.with_state::<StackQuery, _>(|stacks| {
			let stack = stacks.resolve(root);
			let local = stacks
				.default_secret_store(ServiceAccess::Local, root, stack.clone())
				.unwrap();
			local.id().xpect_eq("document");
			local.describe().xpect_contains("app--prod.toml");
			// the test launch is local, so the plain resolution agrees
			stacks.secret_store(root).unwrap().id().xpect_eq("document");
			#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
			{
				let remote = stacks
					.default_secret_store(ServiceAccess::Remote, root, stack)
					.unwrap();
				remote.id().xpect_eq("ssm");
				remote
					.region()
					.unwrap()
					.xpect_eq(stacks.resolve(root).region().clone());
			}
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
			fn address(
				&self,
				stack: &ResolvedStack,
				secret: &SecretRef,
			) -> SmolStr {
				self.inner.address(stack, secret)
			}
			/// The first read misses, as the loser's did.
			fn get(
				&self,
				stack: ResolvedStack,
				secret: SecretRef,
			) -> SendBoxedFuture<Result<Option<String>>> {
				use core::sync::atomic::Ordering;
				match self.misses.fetch_sub(1, Ordering::Relaxed) > 0 {
					true => Box::pin(async { None.xok() }),
					false => self.inner.get(stack, secret),
				}
			}
			fn create(
				&self,
				stack: ResolvedStack,
				secret: SecretRef,
				value: SmolStr,
				meta: SecretMeta,
			) -> SendBoxedFuture<Result> {
				self.inner.create(stack, secret, value, meta)
			}
			fn overwrite(
				&self,
				stack: ResolvedStack,
				secret: SecretRef,
				value: SmolStr,
				meta: SecretMeta,
			) -> SendBoxedFuture<Result> {
				self.inner.overwrite(stack, secret, value, meta)
			}
			fn list(
				&self,
				stack: ResolvedStack,
			) -> SendBoxedFuture<Result<Vec<SecretEntry>>> {
				self.inner.list(stack)
			}
			fn delete(
				&self,
				stack: ResolvedStack,
				secrets: Vec<SecretRef>,
			) -> SendBoxedFuture<Result<Vec<SecretRef>>> {
				self.inner.delete(stack, secrets)
			}
		}
		let stack = Stack::new("app").resolve(&PackageConfig::default());
		let inner = memory_store(&stack);
		let secret = SecretRef::new("db-password");
		// the winner minted first
		SecretStore::new(inner.clone())
			.create(&stack, &secret, "winner", None, Rotation::Remint)
			.await
			.unwrap();
		let store = SecretStore::new(Racing {
			inner,
			misses: Arc::new(1.into()),
		});
		let (value, minted) = store
			.ensure(&stack, &secret, None, Rotation::Remint, async || {
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
