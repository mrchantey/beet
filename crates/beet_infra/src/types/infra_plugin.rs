use beet_core::prelude::*;





#[derive(Default)]
pub struct InfraPlugin;

impl Plugin for InfraPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		#[cfg(feature = "cli")]
		app.init_plugin::<beet_router::prelude::BeetRouterPlugin>();
	}
}
