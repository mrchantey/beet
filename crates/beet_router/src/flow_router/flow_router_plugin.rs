use beet_core::prelude::*;
use beet_flow::prelude::*;



pub struct FlowRouterPlugin;

impl Plugin for FlowRouterPlugin {
	fn build(&self, app: &mut App) { app.init_plugin(BeetFlowPlugin); }
}





#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() { true.xpect_false(); }
}
