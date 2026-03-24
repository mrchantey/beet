use beet_core::prelude::*;

#[derive(Default)]
pub struct ActorPlugin {}

impl Plugin for ActorPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<AsyncPlugin>(); }
}
