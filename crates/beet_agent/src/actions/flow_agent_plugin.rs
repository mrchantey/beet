use beet_core::prelude::*;
use beet_router::prelude::*;

#[derive(Default)]
pub struct FlowAgentPlugin;

impl Plugin for FlowAgentPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<RouterPlugin>(); }
}
