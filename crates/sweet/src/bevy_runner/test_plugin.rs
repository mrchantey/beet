use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

pub struct TestPlugin;

impl Plugin for TestPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<ControlFlowPlugin>()
			.init_plugin::<AsyncPlugin>()
			.add_systems(Startup, start_test_runner);
	}
}
