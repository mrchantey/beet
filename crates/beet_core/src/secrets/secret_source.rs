//! Where a running process reads one secret.

use crate::prelude::*;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::fmt;

/// Where a running process reads one secret: the launch's environment (the
/// records `<Secrets>` loads from `secrets.toml`) or a deployment's secret
/// store, so a consumer never knows which.
///
/// The erased-provider pattern (`AGENTS.md`): a declaration lands this handle
/// on its own entity (`beet_infra`'s `<SecretBlock/>`, picking a provider by
/// the launch's `ServiceAccess`) and a consumer names that entity by relation
/// and reads it through [`resolve`](Self::resolve), ie an ssh server's host
/// key. [`Debug`] prints where the value lives, never the value.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// async_ext::block_on(async {
/// 	// an unset variable is absence, not an error: the consumer's default applies
/// 	SecretSource::env("BEET_DOC_UNSET_SECRET")
/// 		.get()
/// 		.await?
/// 		.xpect_none();
/// 	Ok::<_, BevyError>(())
/// })
/// .unwrap();
/// ```
#[derive(Clone, Component)]
pub struct SecretSource {
	provider: Arc<dyn SecretSourceProvider>,
}

impl fmt::Debug for SecretSource {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SecretSource")
			.field("provider", &self.provider.id())
			.field("at", &self.provider.describe())
			.finish()
	}
}

impl SecretSource {
	/// The erased handle over `provider`.
	pub fn new(provider: impl SecretSourceProvider) -> Self {
		Self {
			provider: Arc::new(provider),
		}
	}

	/// The launch's environment variable `key`, absent when unset: the local
	/// meaning of a secret the launch loads from its secrets document.
	pub fn env(key: impl Into<SmolStr>) -> Self {
		Self::new(EnvSecretSource { key: key.into() })
	}

	/// The provider's id, ie `env`.
	pub fn id(&self) -> &'static str { self.provider.id() }

	/// Where the value lives, for a log: never the value.
	pub fn describe(&self) -> String { self.provider.describe() }

	/// The value, `None` when this source holds none and the consumer's own
	/// default applies. A source whose absence is a fault (a deployed secret
	/// the deploy never mirrored) errors instead, naming what mints it.
	pub async fn get(&self) -> Result<Option<String>> {
		self.provider.get().await
	}

	/// The source a declaration landed on `target`, the entity a consumer's
	/// relation names. A declaration's runtime half lands through the command
	/// queue, so this backs off before failing, and the failure names the
	/// entity so an operator can see which declaration answered nothing.
	#[cfg(feature = "bevy_async")]
	pub async fn resolve(world: &AsyncWorld, target: Entity) -> Result<Self> {
		// the waits up front, so no rng is held across an await and the
		// future stays `Send` for a listener spawned off-thread
		let waits = Backoff::default()
			.with_max_attempts(5)
			.iter()
			.map(|frame| frame.next_attempt)
			.collect::<alloc::vec::Vec<_>>();
		for wait in waits {
			if let Ok(source) =
				world.entity(target).get::<Self, _>(Self::clone).await
			{
				return Ok(source);
			}
			if let Some(wait) = wait {
				time_ext::sleep(wait).await;
			}
		}
		bevybail!(
			"entity {target} has no `SecretSource`: name a secret declaration, \
			ie `<SecretBlock/>`, which lands one"
		)
	}
}

/// One way of reading a secret, see [`SecretSource`].
pub trait SecretSourceProvider: 'static + Send + Sync {
	/// A boxed clone, so a default method can own the provider across an
	/// await.
	fn box_clone(&self) -> Box<dyn SecretSourceProvider>;

	/// Stable provider discriminator, ie `env`.
	fn id(&self) -> &'static str;

	/// Where the value lives, for a log: never the value.
	fn describe(&self) -> String;

	/// See [`SecretSource::get`].
	fn get(&self) -> SendBoxedFuture<Result<Option<String>>>;
}

/// The launch's environment variable of one name, see [`SecretSource::env`].
#[derive(Debug, Clone)]
struct EnvSecretSource {
	key: SmolStr,
}

impl SecretSourceProvider for EnvSecretSource {
	fn box_clone(&self) -> Box<dyn SecretSourceProvider> {
		Box::new(self.clone())
	}

	fn id(&self) -> &'static str { "env" }

	fn describe(&self) -> String {
		alloc::format!("the launch's `{}`", self.key)
	}

	fn get(&self) -> SendBoxedFuture<Result<Option<String>>> {
		let value = env_ext::var(&self.key).ok().map(String::from);
		Box::pin(async move { value.xok() })
	}
}
