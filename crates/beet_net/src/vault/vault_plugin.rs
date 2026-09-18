//! Registration for the vault verbs.

use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the `vault` verbs and `<VaultRoutes/>`, so an entry names them
/// by tag. A build without the `vault` feature loads a document naming them
/// with inert `UnregisteredTag` entities in their place.
#[derive(Default)]
pub struct VaultPlugin;

impl Plugin for VaultPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StorePlugin>()
			.register_template::<VaultRoutes>()
			.register_type::<VaultKeygen>()
			.register_type::<VaultEncrypt>()
			.register_type::<VaultDecrypt>()
			.register_type::<VaultRekey>();
		#[cfg(not(target_arch = "wasm32"))]
		app.register_type::<VaultBackup>()
			.register_type::<VaultRestoreIdentity>();
	}
}
