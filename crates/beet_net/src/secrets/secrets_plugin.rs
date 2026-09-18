//! Registration for the secrets surface.

use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the verbs and `<SecretsRoutes/>`, so an entry names them by
/// tag. A build without the `secrets` feature loads a document naming them
/// with inert `UnregisteredTag` entities in their place.
#[derive(Default)]
pub struct SecretsPlugin;

impl Plugin for SecretsPlugin {
	fn build(&self, app: &mut App) {
		app.register_template::<SecretsRoutes>()
			.register_type::<SecretsKeygen>()
			.register_type::<SecretsCheck>()
			.register_type::<SecretsEncrypt>()
			.register_type::<SecretsDecrypt>()
			.register_type::<SecretsRekey>();
		#[cfg(not(target_arch = "wasm32"))]
		app.register_type::<SecretsBackup>()
			.register_type::<SecretsRestoreIdentity>();
	}
}
