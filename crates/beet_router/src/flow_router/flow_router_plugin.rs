use beet_core::prelude::*;
use beet_flow::prelude::*;



pub struct FlowRouterPlugin;

impl Plugin for FlowRouterPlugin {
	fn build(&self, app: &mut App) { app.init_plugin(BeetFlowPlugin); }
}
