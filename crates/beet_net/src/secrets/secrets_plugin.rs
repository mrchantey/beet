//! Registration for the secrets document surface.

use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the `<Secrets>` declaration and its load, the `secrets` verbs
/// and `<SecretsRoutes/>`, and the vault layer under them ([`VaultPlugin`]),
/// so an entry names them by tag. A build without the `vault` feature still
/// knows the `<Secrets>` declaration (`RouterPlugin` registers it, inert),
/// and loads the verbs a document names as inert entities holding each as `Unregistered`.
#[derive(Default)]
pub struct SecretsPlugin;

impl Plugin for SecretsPlugin {
	fn build(&self, app: &mut App) {
		app.install_types::<VaultPlugin>()
			.register_type::<Secrets>()
			.register_type::<SecretRole>()
			.add_observer(load_on_insert)
			.install_types::<SecretsRoutes>()
			.register_type::<SecretsCheck>()
			.register_type::<SecretsLs>()
			.register_type::<SecretsGet>()
			.register_type::<SecretsSet>()
			.register_type::<SecretsRm>()
			.register_type::<SecretsRekey>();
		#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
		app.register_type::<SecretsExec>();
		// mounted at an entry's TOP level rather than under `secrets`, so the
		// everyday credential command is `beet aws` and the rare one is
		// `beet admin`
		#[cfg(all(
			feature = "fs",
			feature = "aws",
			not(target_arch = "wasm32")
		))]
		app.register_type::<AwsExec>()
			.register_type::<AdminElevate>();
	}
}
