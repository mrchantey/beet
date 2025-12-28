use super::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;

#[derive(Default)]
pub struct TestPlugin;

impl Plugin for TestPlugin {
	fn build(&self, app: &mut App) {
		#[cfg(target_arch = "wasm32")]
		console_error_panic_hook::set_once();

		app.init_plugin::<ControlFlowPlugin>()
			.init_plugin::<AsyncPlugin>()
			.insert_schedule_before(Update, RunTests)
			.add_systems(
				RunTests,
				(
					filter_tests,
					(run_tests_series, run_non_send_tests_series),
					// #[cfg(not(test))]
					log_incremental,
					insert_final_outcome,
					exit_on_done,
				)
					.chain(),
			);
	}
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RunTests;
