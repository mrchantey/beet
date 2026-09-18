//! Registration for the secrets document surface.

use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the `<Secrets>` declaration and its load, the `secrets` verbs
/// and `<SecretsRoutes/>`, and the vault layer under them ([`VaultPlugin`]),
/// so an entry names them by tag. A build without the `vault` feature loads
/// a document naming them with inert `UnregisteredTag` entities in their
/// place.
#[derive(Default)]
pub struct SecretsPlugin;

impl Plugin for SecretsPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<VaultPlugin>()
			.register_type::<Secrets>()
			.register_type::<SecretRole>()
			.add_observer(load_on_insert)
			.register_template::<SecretsRoutes>()
			.register_type::<SecretsCheck>()
			.register_type::<SecretsLs>()
			.register_type::<SecretsGet>()
			.register_type::<SecretsSet>()
			.register_type::<SecretsRm>()
			.register_type::<SecretsRekey>();
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		app.register_type::<SecretsExec>();
	}
}
