//! Registration for the secrets surface.

use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the vault declarations ([`Vault`], [`AgeRecipients`]) and, under
/// the `secrets` feature, the verbs and `<SecretsRoutes/>`. The declarations
/// register in every std build: a document naming a `<Vault>` loads whole in
/// a lean binary, with only the verbs missing.
#[derive(Default)]
pub struct SecretsPlugin;

impl Plugin for SecretsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Vault>()
			.register_type::<AgeRecipients>();
		// the verbs and their mount, so an entry names them by tag
		#[cfg(feature = "secrets")]
		app.register_template::<SecretsRoutes>()
			.register_type::<SecretsKeygen>()
			.register_type::<SecretsCheck>()
			.register_type::<SecretsLs>()
			.register_type::<SecretsGet>()
			.register_type::<SecretsSet>()
			.register_type::<SecretsRm>()
			.register_type::<SecretsImport>()
			.register_type::<SecretsEnv>()
			.register_type::<SecretsRekey>();
		#[cfg(all(feature = "secrets", not(target_arch = "wasm32")))]
		app.register_type::<SecretsBackup>()
			.register_type::<SecretsRestoreIdentity>();
		#[cfg(all(
			feature = "secrets",
			feature = "fs",
			not(target_arch = "wasm32")
		))]
		app.register_type::<SecretsExec>();
	}
}
