use beet_core::prelude::*;
use beet_router::prelude::*;

#[derive(Default)]
pub struct ActorPlugin {}

impl Plugin for ActorPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<RouterPlugin>();
	}
}
