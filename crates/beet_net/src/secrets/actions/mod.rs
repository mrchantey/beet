//! The `secrets` verbs, one file per verb, each a route action carrying its
//! own params type so `--help` documents its flags. Every vault verb resolves
//! its vault through [`VaultHandle::resolve`] and its identity through
//! `AgeIdentityFile::require`, failing with the `keygen` guidance when none
//! resolves. Values never reach a log: `get` and `env` print them
//! deliberately as their response, everything else answers with labels,
//! counts and paths.

#[cfg(not(target_arch = "wasm32"))]
mod backup;
mod check;
mod env;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod exec;
mod get;
mod import;
mod keygen;
mod ls;
mod rekey;
#[cfg(not(target_arch = "wasm32"))]
mod restore_identity;
mod rm;
mod secrets_routes;
mod set;

#[cfg(not(target_arch = "wasm32"))]
pub use backup::*;
pub use check::*;
pub use env::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use exec::*;
pub use get::*;
pub use import::*;
pub use keygen::*;
pub use ls::*;
pub use rekey::*;
#[cfg(not(target_arch = "wasm32"))]
pub use restore_identity::*;
pub use rm::*;
pub use secrets_routes::*;
pub use set::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// The `--vault` every vault verb takes.
#[derive(Reflect)]
pub(crate) struct VaultParams {
	/// The vault: a declared `<Vault>` label, or the path or store uri of an
	/// undeclared file (`~/personal.toml.age`,
	/// `s3://bucket/secrets/x.toml.age`). Defaults to `.env`, the entry's
	/// `.env.age`.
	pub vault: Option<String>,
}

impl VaultParams {
	/// The vault the request names, resolved from `caller`.
	pub(crate) async fn resolve(
		request: &Request,
		caller: &AsyncEntity,
	) -> Result<VaultHandle> {
		let params = request.parse_params::<Self>()?;
		VaultHandle::resolve(caller, params.vault.as_deref()).await
	}
}

/// The `:key` segment of `get/:key`, `set/:key` and `rm/:key`: a flat key
/// on an env vault, a dotted path (`secrets.dkim.value`) on a tree.
pub(crate) fn key_param(request: &Request) -> Result<SmolStr> {
	request
		.get_param("key")
		.map(str::trim)
		.filter(|key| !key.is_empty())
		.map(SmolStr::new)
		.ok_or_else(|| {
			bevyhow!(
				"a key is required, ie `secrets/{} OPENAI_API_KEY`",
				request.path().last().map(SmolStr::as_str).unwrap_or("get")
			)
		})
}

#[cfg(test)]
pub(crate) mod test_support {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use bevy::platform::sync::LazyLock;

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

	/// A world holding a repo store with a declared vault of each tree kind
	/// under inherited recipients, and the undeclared `.env` default: the
	/// fixture every verb test runs against.
	pub struct VerbWorld {
		pub world: World,
		pub root: Entity,
		pub identity: AgeIdentity,
		pub store: BlobStore,
	}

	impl VerbWorld {
		/// Build the fixture, forcing the process identity into place.
		pub fn new() -> Self {
			let identity = IDENTITY.clone();
			let store = BlobStore::temp();
			let mut world = (AsyncPlugin, StorePlugin).into_world();
			let root = world
				.spawn((store.clone(), RepoStore, children![(
					AgeRecipients(vec![identity.to_recipient()]),
					children![
						Vault::new("secrets/mail.toml.age").with_label("mail"),
						Vault::new("secrets/notes.json.age")
							.with_label("notes"),
					]
				)]))
				.id();
			world.flush();
			Self {
				world,
				root,
				identity,
				store,
			}
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

		/// A `set` of `key` in `vault`, the write most tests start from.
		pub async fn set(&mut self, vault: &str, key: &str, value: &str) {
			self.call_str(
				SecretsSet,
				Request::get("/")
					.with_param("vault", vault)
					.with_param("key", key)
					.with_param("value", value),
			)
			.await
			.unwrap();
		}
	}
}
