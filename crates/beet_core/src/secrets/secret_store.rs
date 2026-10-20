//! The provider-agnostic seam every secret a process reads or a deploy mints
//! lives behind.

use crate::prelude::*;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt;

/// A secret store scoped to one [`SecretScope`]: the launch's environment
/// ([`EnvSecretStore`]), AWS parameter store and a secrets document in
/// `beet_infra`, 1Password or anything else downstream. Every method takes
/// a [`SecretRef`] label alone and the provider composes its own address
/// from the pair, so a label (`dkim-example-com`) never couples to a
/// provider and an export restores into a different provider or region
/// unchanged. Another scope of the same provider is
/// [`for_scope`](Self::for_scope), the `with_subdir` of the seam.
///
/// The erased-provider pattern (`AGENTS.md`): an `Arc<dyn SecretStoreProvider>`
/// cloned by every consumer, landed on the declaring entity by the attach
/// observer of its declaration and resolved by ancestry, an
/// `AncestorQuery<&SecretStore>` as a `BlobStore` is; a
/// process reading one secret holds the store beside its key, a
/// [`StoredSecret`]. [`Debug`] prints the provider id and scope, never a
/// value; a consumer that prints one says so in its own docs.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// async_ext::block_on(async {
/// 	// SAFETY: a variable no other doctest reads
/// 	unsafe { env_ext::set_var("BEET_DOC_SECRET", "hunter2") }?;
/// 	let store = SecretStore::new(EnvSecretStore);
/// 	store
/// 		.get(&SecretRef::new("BEET_DOC_SECRET"))
/// 		.await?
/// 		.xpect_eq(Some("hunter2".into()));
/// 	// absence is `None`, and a caller that needs the value names its source
/// 	store
/// 		.require(&SecretRef::new("BEET_DOC_UNSET"), || "set the record".into())
/// 		.await
/// 		.unwrap_err()
/// 		.to_string()
/// 		.xpect_contains("BEET_DOC_UNSET");
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
			.field("scope", &self.provider.scope().to_string())
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

	/// The provider's id, ie `env`, `ssm`, `document`.
	pub fn id(&self) -> &'static str { self.provider.id() }

	/// Where the provider lives, for an export's `origin.region`.
	pub fn region(&self) -> Option<SmolStr> { self.provider.region() }

	/// How a log names this store: its id and region.
	pub fn describe(&self) -> String { self.provider.describe() }

	/// The scope this handle composes its addresses under.
	pub fn scope(&self) -> &SecretScope { self.provider.scope() }

	/// The same provider scoped to `scope`: how a drill reads its source
	/// stage's credential through its own store. A provider that cannot
	/// serve another scope (a document holds one) errors naming both.
	pub fn for_scope(&self, scope: &SecretScope) -> Result<Self> {
		Self {
			provider: Arc::from(self.provider.for_scope(scope)?),
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
	/// ([`ensure`](Self::ensure)). `note` and `roll` are stored where the
	/// provider can (SSM's description, a document's index); a note is never
	/// a secret, and a roll is required so no secret arrives un-rollable.
	pub async fn create(
		&self,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		roll: SecretRoll,
	) -> Result {
		self.provider
			.create(
				secret.clone(),
				value.into(),
				SecretMeta::new(note, Some(roll)),
			)
			.await
	}

	/// Create or replace `secret`. The opposite posture to
	/// [`create`](Self::create), for the opposite ownership: a value some
	/// OTHER system mints and this store mirrors (the bootstrap admin
	/// credential a fresh mail server returns exactly once, a pin read off a
	/// box, a record of the deploying entry's secrets document), where an
	/// existing value is by definition stale.
	pub async fn overwrite(
		&self,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		roll: Option<SecretRoll>,
	) -> Result {
		self.provider
			.overwrite(
				secret.clone(),
				value.into(),
				SecretMeta::new(note, roll),
			)
			.await
	}

	/// The stored metadata of one secret, `None` when it does not exist.
	pub async fn meta(&self, secret: &SecretRef) -> Result<Option<SecretMeta>> {
		self.provider.meta(secret.clone()).await
	}

	/// Mirror a value another system derives (a public half, a pin read off
	/// a box, a record of the deploying entry's document):
	/// [`overwrite`](Self::overwrite) when the stored value or its metadata
	/// differs from what is declared, nothing when both match. Answers
	/// whether it wrote, so a caller logs the change and not the steady
	/// state; a stale note or roll converges around an unchanged value
	/// exactly as [`ensure`](Self::ensure) does.
	pub async fn converge(
		&self,
		secret: &SecretRef,
		value: &str,
		note: Option<&str>,
		roll: Option<SecretRoll>,
	) -> Result<bool> {
		let declared = SecretMeta::new(note, roll.clone());
		if self.get(secret).await?.as_deref() == Some(value)
			&& self.meta(secret).await?.as_ref() == Some(&declared)
		{
			return false.xok();
		}
		self.overwrite(secret, value, note, roll).await?;
		true.xok()
	}

	/// Create-if-missing: the existing value, else `generate` created and
	/// its value, else (the loser of a race) the winner's value re-read.
	/// Answers the value and whether this call minted it. The one shape
	/// every generated credential in a stack takes, so no consumer can
	/// roll one by accident. An existing value whose stored note or roll
	/// differs from the declared is rewritten unchanged with the declared
	/// metadata, since the mint site is where both are declared and the
	/// store only mirrors them.
	pub async fn ensure(
		&self,
		secret: &SecretRef,
		note: Option<&str>,
		roll: SecretRoll,
		generate: impl AsyncFnOnce() -> Result<String>,
	) -> Result<(String, bool)> {
		let address = self.address(secret);
		if let Some(value) = self.get(secret).await? {
			let declared = SecretMeta::new(note, Some(roll));
			if self.meta(secret).await?.as_ref() != Some(&declared) {
				self.overwrite(secret, &value, note, declared.roll).await?;
				info!("converged the metadata of secret {address}");
			}
			return (value, false).xok();
		}
		let generated = generate().await?;
		match self.create(secret, &generated, note, roll).await {
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

	/// Every secret of the scope, with its metadata and never a value.
	pub async fn list(&self) -> Result<Vec<SecretEntry>> {
		self.provider.list().await
	}

	/// Every secret of the scope with its value: what an export reads.
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

/// A secret store backend scoped to one [`SecretScope`], see
/// [`SecretStore`].
pub trait SecretStoreProvider: 'static + Send + Sync {
	/// A boxed clone, so a default method can own the provider across an
	/// await.
	fn box_clone(&self) -> Box<dyn SecretStoreProvider>;

	/// Stable provider discriminator, ie `env`, `ssm`, `document`.
	fn id(&self) -> &'static str;

	/// The provider's region, where it has one.
	fn region(&self) -> Option<SmolStr> { None }

	/// Where the store lives, for a log: the id and region by default.
	fn describe(&self) -> String {
		match self.region() {
			Some(region) => format!("{} ({region})", self.id()),
			None => self.id().to_string(),
		}
	}

	/// The scope this provider composes its addresses under.
	fn scope(&self) -> &SecretScope;

	/// See [`SecretStore::for_scope`].
	fn for_scope(
		&self,
		scope: &SecretScope,
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
					roll: entry.roll,
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
	/// How the secret rolls, stored at mint; absent on one parked by hand.
	pub roll: Option<SecretRoll>,
}

/// What a write stores beside a value: the note and the roll, which a
/// provider keeps where it can (SSM's description, a document's index).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SecretMeta {
	/// A plaintext note, never a secret: what the value is for.
	pub note: Option<SmolStr>,
	/// How the value rolls; every mint declares one, a restore carries
	/// the record's.
	pub roll: Option<SecretRoll>,
}

impl SecretMeta {
	/// The metadata of `note` and `roll`.
	pub fn new(note: Option<&str>, roll: Option<SecretRoll>) -> Self {
		Self {
			note: note.map(SmolStr::new),
			roll,
		}
	}

	/// The one string a provider with a single description field holds
	/// (a parameter's description): the roll, then ` :: `, then the note,
	/// either half optional, so a listing reads both back
	/// ([`parse_description`](Self::parse_description)) and a human reads
	/// it in the console.
	pub fn description(&self) -> String {
		match (&self.roll, &self.note) {
			(Some(roll), Some(note)) => format!("{roll} :: {note}"),
			(Some(roll), None) => roll.to_string(),
			(None, Some(note)) => note.to_string(),
			(None, None) => String::new(),
		}
	}

	/// The inverse of [`description`](Self::description): a leading half
	/// that parses as a roll is one, the rest is the note.
	pub fn parse_description(text: &str) -> Self {
		let text = text.trim();
		let note = |text: &str| {
			Some(SmolStr::new(text)).filter(|note| !note.is_empty())
		};
		match text.split_once(" :: ") {
			Some((head, rest)) if let Ok(roll) = head.parse::<SecretRoll>() => {
				Self {
					note: note(rest.trim()),
					roll: Some(roll),
				}
			}
			_ => match text.parse::<SecretRoll>() {
				Ok(roll) => Self {
					note: None,
					roll: Some(roll),
				},
				Err(_) => Self {
					note: note(text),
					roll: None,
				},
			},
		}
	}
}

/// The one failure a consumer matches on: [`SecretStore::create`] found the
/// secret already there, so the caller re-reads rather than overwrites.
#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
	/// The secret at `address` exists.
	#[error("secret {address} already exists")]
	AlreadyExists {
		/// The provider's address of the secret.
		address: SmolStr,
	},
}

impl SecretStoreError {
	/// Whether `err` is the create conflict, ie somebody else won the race.
	pub fn is_already_exists(err: &BevyError) -> bool {
		matches!(err.downcast_ref::<Self>(), Some(Self::AlreadyExists { .. }))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A description carries the roll and the note and reads both back,
	/// each half on its own.
	#[crate::test]
	fn descriptions_carry_the_roll_and_the_note() {
		let replace = SecretRoll::replace("cloudflare_account_token.x");
		let meta = SecretMeta::new(Some("r2 token"), Some(replace.clone()));
		let text = meta.description();
		text.as_str()
			.xpect_eq("replace:cloudflare_account_token.x :: r2 token");
		SecretMeta::parse_description(&text).xpect_eq(meta);
		SecretMeta::parse_description("remint")
			.xpect_eq(SecretMeta::new(None, Some(SecretRoll::Remint)));
		SecretMeta::parse_description("just a note")
			.xpect_eq(SecretMeta::new(Some("just a note"), None));
		SecretMeta::parse_description("").xpect_eq(SecretMeta::default());
		// a note happening to contain the separator stays a note
		SecretMeta::parse_description("weekly :: by hand")
			.xpect_eq(SecretMeta::new(Some("weekly :: by hand"), None));
		// a note with a colon and a `>` in it, as a mint site writes one
		let manual = SecretMeta::new(
			Some("R2 token for bucket cold-backups: access key id"),
			Some(SecretRoll::manual(
				"dash.cloudflare.com/profile/api-tokens > Create Token > beet-deploy",
			)),
		);
		SecretMeta::parse_description(&manual.description()).xpect_eq(manual);
	}

	#[crate::test]
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
