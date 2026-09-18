//! The `secrets` verbs, one file per verb, each a route action carrying its
//! own params type so `--help` documents its flags. The record verbs (`ls`,
//! `get`, `set`, `rm`, `rekey`, `check`, `exec`) act on a secrets document
//! named by `--vault=<label or path>`, the declared one by default
//! ([`DocumentParams`]); the file verbs (`encrypt`, `decrypt`) act on any
//! age file by path ([`VaultParams`]). Every verb but `ls` resolves its
//! identity through `AgeIdentityFile::require`, failing with the `keygen`
//! guidance when none resolves. Values never reach a log: `get` and
//! `decrypt` print theirs deliberately as their response, everything else
//! answers with names, counts and paths.

#[cfg(not(target_arch = "wasm32"))]
mod backup;
mod check;
mod decrypt;
mod encrypt;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod exec;
mod get;
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
pub use decrypt::*;
pub use encrypt::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use exec::*;
pub use get::*;
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

/// The `--vault` every document verb takes.
#[derive(Reflect)]
pub(crate) struct DocumentParams {
	/// The document: a declared `<Secrets>` label, or the path or store uri
	/// of an undeclared file (`~/personal.toml.age`,
	/// `s3://bucket/secrets/x.toml.age`). Defaults to the one declared
	/// document, or `secrets.toml.age` beside the entry when none is.
	pub vault: Option<String>,
}

impl DocumentParams {
	/// The document the request names, resolved from `caller`.
	pub(crate) async fn resolve(
		request: &Request,
		caller: &AsyncEntity,
	) -> Result<VaultHandle> {
		let params = request.parse_params::<Self>()?;
		VaultHandle::resolve_document(caller, params.vault.as_deref()).await
	}
}

/// The `:name` segment of `get/:name`, `set/:name` and `rm/:name`.
pub(crate) fn name_param(request: &Request) -> Result<SmolStr> {
	request
		.get_param("name")
		.map(str::trim)
		.filter(|name| !name.is_empty())
		.map(SmolStr::new)
		.ok_or_else(|| {
			bevyhow!(
				"a record name is required, ie `secrets/{} OPENAI_API_KEY`",
				request.path().last().map(SmolStr::as_str).unwrap_or("get")
			)
		})
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
	/// its own as the repo store, addressable by uri too: the fixture every
	/// verb test runs against. The store handle is held here because a
	/// memory backing lives only as long as a handle does, and a verb given
	/// a uri resolves it afresh. With nothing declared the document verbs
	/// land on `secrets.toml.age` in the repo store.
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
			let mut world = (AsyncPlugin, SecretsPlugin).into_world();
			let uri = format!(
				"memory://verbs-{}",
				STORES.fetch_add(1, Ordering::Relaxed)
			);
			let store =
				StoreProvider::from_uri(&StoreUri::parse(&uri).unwrap())
					.unwrap()
					.into_blob_store();
			let root = world.spawn((store.clone(), RepoStore)).id();
			world.flush();
			Self {
				world,
				root,
				identity,
				store,
				uri,
			}
		}

		/// The identity file holding the fixture's identity alone.
		pub fn identities(&self) -> AgeIdentityFile {
			let mut file = AgeIdentityFile::default();
			file.push(self.identity.clone());
			file
		}

		/// The default document, `secrets.toml.age` in the repo store, empty
		/// until written.
		pub async fn document(&self) -> SecretsDocument {
			self.vault(SecretsDocument::DEFAULT_PATH)
				.read_or_new_document()
				.await
				.unwrap()
		}

		/// Write `record` with `value` into the default document as the
		/// fixture's identity: the state most record tests start from.
		pub async fn set(&self, name: &str, value: &str, record: SecretRecord) {
			let mut document = self.document().await;
			document
				.set(&self.identities(), name, value, record)
				.unwrap();
			self.vault(SecretsDocument::DEFAULT_PATH)
				.write_document(&document)
				.await
				.unwrap();
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
