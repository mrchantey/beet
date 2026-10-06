use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin that registers all beet_net types for world serialization.
///
/// Includes [`StorePlugin`] for typed store and blob registration,
/// [`AtprotoPlugin`] for account declarations under `json` and, under
/// the `vault` feature, `SecretsPlugin` (which brings `VaultPlugin`) for the
/// `<Secrets>` declaration and the `vault` and `secrets` verbs.
#[derive(Default)]
pub struct NetPlugin;

impl Plugin for NetPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<StorePlugin>();
		// account declarations load in every build; their repos need `atproto`
		#[cfg(feature = "json")]
		app.init_plugin::<AtprotoPlugin>();
		#[cfg(feature = "vault")]
		app.init_plugin::<SecretsPlugin>();
		// the read a route asks of a declared index, ie
		// `<Route path="query/senders" {(SqlSelect{sql:".."}, StoreRef($index))}/>`
		#[cfg(feature = "sqlite")]
		app.register_type::<SqlSelect>()
			.register_type::<RowFormat>();
	}
}
