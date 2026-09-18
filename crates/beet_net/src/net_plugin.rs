use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers all beet_net types for world serialization.
///
/// Includes [`StorePlugin`] for typed store and blob registration and, under
/// the `vault` feature, `SecretsPlugin` (which brings `VaultPlugin`) for the
/// `<Secrets>` declaration and the `vault` and `secrets` verbs.
#[derive(Default)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StorePlugin>();
		#[cfg(feature = "vault")]
		app.init_plugin::<SecretsPlugin>();
	}
}
