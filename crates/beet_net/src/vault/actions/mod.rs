//! The `vault` verbs, one file per verb, each a route action carrying its
//! own params type so `--help` documents its flags: the identity verbs
//! (`keygen`, `backup`, `restore-identity`) and the file verbs (`encrypt`,
//! `decrypt`, `rekey`), which take `--vault=<path or store uri>` and act on
//! one age file. Every verb but `keygen` resolves its identity through
//! `AgeIdentityFile::require`, failing with the `keygen` guidance when none
//! resolves. Values never reach a log: `decrypt` prints its plaintext
//! deliberately as its response, everything else answers with counts and
//! paths.

#[cfg(not(target_arch = "wasm32"))]
mod backup;
mod decrypt;
mod encrypt;
mod keygen;
mod rekey;
#[cfg(not(target_arch = "wasm32"))]
mod restore_identity;
mod vault_routes;

#[cfg(not(target_arch = "wasm32"))]
pub use backup::*;
pub use decrypt::*;
pub use encrypt::*;
pub use keygen::*;
pub use rekey::*;
#[cfg(not(target_arch = "wasm32"))]
pub use restore_identity::*;
pub use vault_routes::*;

use crate::prelude::*;
use beet_core::prelude::*;

/// The `--vault` every file verb takes.
#[derive(Reflect)]
pub(crate) struct VaultParams {
	/// The age file: a filesystem path (`~/keys/id_ed25519.age`,
	/// `dir/x.age`) or a store uri ending in the file
	/// (`s3://bucket/certs/x.pem.age`).
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
