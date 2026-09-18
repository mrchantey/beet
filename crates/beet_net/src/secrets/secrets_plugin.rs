//! Registration for the secrets surface.

use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the `<Secrets>` declaration and its load, the verbs and
/// `<SecretsRoutes/>`, so an entry names them by tag. A build without the
/// `secrets` feature loads a document naming them with inert
/// `UnregisteredTag` entities in their place.
#[derive(Default)]
pub struct SecretsPlugin;

impl Plugin for SecretsPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StorePlugin>()
			.register_type::<Secrets>()
			.register_type::<SecretRole>()
			.add_observer(load_on_insert)
			.register_template::<SecretsRoutes>()
			.register_type::<SecretsKeygen>()
			.register_type::<SecretsCheck>()
			.register_type::<SecretsLs>()
			.register_type::<SecretsGet>()
			.register_type::<SecretsSet>()
			.register_type::<SecretsRm>()
			.register_type::<SecretsRekey>()
			.register_type::<SecretsEncrypt>()
			.register_type::<SecretsDecrypt>();
		#[cfg(not(target_arch = "wasm32"))]
		app.register_type::<SecretsBackup>()
			.register_type::<SecretsRestoreIdentity>();
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		app.register_type::<SecretsExec>();
	}
}
