use crate::prelude::*;
use beet_core::prelude::*;
use beet_node::prelude::*;

/// Master plugin for `beet_router`, combining all sub-plugins.
///
/// This plugin initializes:
/// - [`AsyncPlugin`] — async command infrastructure
/// - [`DocumentPlugin`] — document field sync (from beet_node)
/// - [`RouterPlugin`] — route tree building observers
/// - [`InterfacePlugin`] — single-active-scene enforcement
/// - [`InputPlugin`] — link click navigation wiring
#[derive(Default)]
pub struct BeetRouterPlugin;

impl Plugin for BeetRouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<DocumentPlugin>()
			.init_plugin::<RouterPlugin>()
			.init_plugin::<NavigatorPlugin>();
	}
}
