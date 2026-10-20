//! One secret a process reads: a store and the key in it.

use crate::prelude::*;
use alloc::string::String;

/// One secret a process reads: a [`SecretStore`] and the [`SecretRef`] in
/// it, landed by a declaration on its own entity and named by a consumer's
/// relation, so the consumer never knows which provider answers. `beet_infra`'s
/// `<SecretBlock/>` lands one by the launch's [`ServiceAccess`]: the launch's
/// environment and the record's name locally ([`EnvSecretStore`]), its
/// stack's store and the secret's label when deployed. An ssh server's host
/// key reads through one (`SshHostKeyRef` in `beet_net`).
///
/// Absence is the consumer's to decide unless the declaration says
/// otherwise: [`get`](Self::get) answers `None` and the consumer's default
/// applies (a local launch without the record), while a secret declared
/// [`required`](Self::required) errors naming what supplies it (a deployed
/// box whose store lacks the value its deploy writes), never a fall back.
/// [`Debug`] prints the store and the key, never a value.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// async_ext::block_on(async {
/// 	let secret = StoredSecret::new(
/// 		SecretStore::new(EnvSecretStore),
/// 		SecretRef::new("BEET_DOC_STORED_UNSET"),
/// 	);
/// 	// an unset record is absence: the consumer's default applies
/// 	secret.get().await?.xpect_none();
/// 	// unless the declaration requires it
/// 	secret
/// 		.required("deploy the stack")
/// 		.get()
/// 		.await
/// 		.unwrap_err()
/// 		.to_string()
/// 		.xpect_contains("deploy the stack");
/// 	Ok::<_, BevyError>(())
/// })
/// .unwrap();
/// ```
#[derive(Debug, Clone, Get, Component)]
pub struct StoredSecret {
	/// The store the value lives in.
	store: SecretStore,
	/// The key in [`store`](Self::store).
	secret: SecretRef,
	/// What supplies the value when its absence is a fault, ie the deploy
	/// that writes it; `None` when absence is legal.
	supplied_by: Option<String>,
}

impl StoredSecret {
	/// `secret` in `store`, absence legal.
	pub fn new(store: SecretStore, secret: SecretRef) -> Self {
		Self {
			store,
			secret,
			supplied_by: None,
		}
	}

	/// The same secret with absence a fault, the error naming `supplied_by`,
	/// the step that supplies the value.
	pub fn required(mut self, supplied_by: impl Into<String>) -> Self {
		self.supplied_by = Some(supplied_by.into());
		self
	}

	/// Where the value lives, for a log: the store and the address, never
	/// the value.
	pub fn describe(&self) -> String {
		alloc::format!(
			"{} {}",
			self.store.describe(),
			self.store.address(&self.secret)
		)
	}

	/// The value, `None` when the store holds none and the secret is not
	/// [`required`](Self::required).
	pub async fn get(&self) -> Result<Option<String>> {
		match &self.supplied_by {
			None => self.store.get(&self.secret).await,
			Some(supplied_by) => self
				.store
				.require(&self.secret, || supplied_by.clone())
				.await
				.map(Some),
		}
	}

	/// The secret a declaration landed on `target`, the entity a consumer's
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
			if let Ok(secret) =
				world.entity(target).get::<Self, _>(Self::clone).await
			{
				return Ok(secret);
			}
			if let Some(wait) = wait {
				time_ext::sleep(wait).await;
			}
		}
		bevybail!(
			"entity {target} has no `StoredSecret`: name a secret declaration, \
			ie `<SecretBlock/>`, which lands one"
		)
	}
}
