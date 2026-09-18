//! The `secrets` verbs, one file per verb, each a route action carrying its
//! own params type so `--help` documents its flags. Every file verb takes
//! `--vault=<path or store uri>` and resolves its identity through
//! `AgeIdentityFile::require`, failing with the `keygen` guidance when none
//! resolves. Values never reach a log: `decrypt` prints its plaintext
//! deliberately as its response, everything else answers with counts and
//! paths.
//!
//! This is the byte layer: a key, a certificate, any file. The record verbs
//! over the secrets document (`ls`, `get`, `set`, `rm`, `exec`) are the next
//! phase.

#[cfg(not(target_arch = "wasm32"))]
mod backup;
mod check;
mod decrypt;
mod encrypt;
mod keygen;
mod rekey;
#[cfg(not(target_arch = "wasm32"))]
mod restore_identity;
mod secrets_routes;

#[cfg(not(target_arch = "wasm32"))]
pub use backup::*;
pub use check::*;
pub use decrypt::*;
pub use encrypt::*;
pub use keygen::*;
pub use rekey::*;
#[cfg(not(target_arch = "wasm32"))]
pub use restore_identity::*;
pub use secrets_routes::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// The `--vault` every file verb takes.
#[derive(Reflect)]
pub(crate) struct VaultParams {
	/// The age file: a filesystem path (`~/personal.toml.age`, `dir/x.age`)
	/// or a store uri ending in the file (`s3://bucket/secrets/x.toml.age`).
	pub vault: String,
}

impl VaultParams {
	/// The file the request names.
	pub(crate) fn resolve(request: &Request) -> Result<VaultHandle> {
		VaultHandle::from_uri(&request.parse_params::<Self>()?.vault)
	}
}

/// The recipients a write encrypts to: `list` (comma separated `age1..`,
/// each validated) when given, else the identity file's own, logged so.
pub(crate) fn write_recipients(
	list: Option<&str>,
	identities: &AgeIdentityFile,
) -> Result<Vec<AgeRecipient>> {
	let Some(list) = list else {
		info!(
			"no `--recipients`, so encrypting to this identity file's own {} \
			recipient(s)",
			identities.len()
		);
		return identities.recipients().xok();
	};
	let recipients = list
		.split(',')
		.map(str::trim)
		.filter(|item| !item.is_empty())
		.map(AgeRecipient::new)
		.collect::<Result<Vec<_>>>()?;
	if recipients.is_empty() {
		bevybail!("`--recipients` names no recipient");
	}
	recipients.xok()
}

#[cfg(test)]
pub(crate) mod test_support {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use bevy::platform::sync::LazyLock;
	use core::sync::atomic::AtomicUsize;
	use core::sync::atomic::Ordering;

	/// The one identity every verb test runs as, set inline through
	/// `BEET_AGE_IDENTITY` exactly once per process (one human per machine):
	/// a per-test value would race the shared environment.
	static IDENTITY: LazyLock<AgeIdentity> = LazyLock::new(|| {
		let identity = AgeIdentity::generate();
		// SAFETY: test-only, set once before any verb test reads it
		unsafe {
			env_ext::set_var(AgeIdentityFile::ENV_VAR, &identity.to_string())
				.unwrap();
		}
		identity
	});

	/// One memory store per fixture, so tests never share a file.
	static STORES: AtomicUsize = AtomicUsize::new(0);

	/// A world to call verbs in, the process identity, and a memory store of
	/// its own addressed by uri: the fixture every verb test runs against.
	/// The store handle is held here because a memory backing lives only as
	/// long as a handle does, and the verbs resolve the uri afresh.
	pub struct VerbWorld {
		pub world: World,
		pub root: Entity,
		pub identity: AgeIdentity,
		store: BlobStore,
		uri: String,
	}

	impl VerbWorld {
		/// Build the fixture, forcing the process identity into place.
		pub fn new() -> Self {
			let identity = IDENTITY.clone();
			let mut world = (AsyncPlugin, StorePlugin).into_world();
			let root = world.spawn_empty().id();
			let uri = format!(
				"memory://verbs-{}",
				STORES.fetch_add(1, Ordering::Relaxed)
			);
			let store =
				StoreProvider::from_uri(&StoreUri::parse(&uri).unwrap())
					.unwrap()
					.into_blob_store();
			Self {
				world,
				root,
				identity,
				store,
				uri,
			}
		}

		/// The uri of `name` in this fixture's store: the `--vault` a verb
		/// takes.
		pub fn uri(&self, name: &str) -> String {
			format!("{}/{name}", self.uri)
		}

		/// The file `name` in this fixture's store.
		pub fn vault(&self, name: &str) -> VaultHandle {
			VaultHandle::new(self.store.clone(), name).unwrap()
		}

		/// Write `plaintext` to `name`, encrypted to the fixture's identity:
		/// the state most tests start from.
		pub async fn write(&self, name: &str, plaintext: &str) {
			self.vault(name)
				.write(plaintext.as_bytes(), &[self.identity.to_recipient()])
				.await
				.unwrap();
		}

		/// Call `verb` spawned under the root with `request`.
		pub async fn call(
			&mut self,
			verb: impl Bundle,
			request: Request,
		) -> Result<Response> {
			let root = self.root;
			self.world
				.spawn((verb, ChildOf(root)))
				.run_async_then(move |entity| async move {
					entity.call::<Request, Response>(request).await
				})
				.await
		}

		/// [`call`](Self::call) and read the response body as text.
		pub async fn call_str(
			&mut self,
			verb: impl Bundle,
			request: Request,
		) -> Result<String> {
			self.call(verb, request).await?.unwrap_str().await.xok()
		}
	}
}
