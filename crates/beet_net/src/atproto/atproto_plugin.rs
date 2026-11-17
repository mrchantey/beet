use crate::prelude::*;
use beet_core::prelude::*;

/// Registers the account declarations every build loads, and under the
/// `atproto` feature the attach that lands each account's [`Pds`] and the
/// [`AppView`] a subtree reads Bluesky through.
#[derive(Default)]
pub struct AtprotoPlugin;

impl Plugin for AtprotoPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<AtprotoAccount>()
			.register_type::<AccountRef>()
			.register_type::<Did>();
		#[cfg(feature = "atproto")]
		app.register_type::<AppView>()
			.add_observer(super::atproto_account::attach_account);
	}
}
