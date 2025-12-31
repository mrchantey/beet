use super::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;


pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	App::new()
		.init_plugin::<JsRuntimePlugin>()
		.add_plugins((MinimalPlugins, TestPlugin))
		.spawn_then((
			Request::from_cli_args(CliArgs::parse_env()).unwrap_or_exit(),
			PathPartial::new("*include?"),
			tests_bundle_borrowed(tests),
		))
		.run()
		.into_exit_native();
}


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
					log_initial,
					filter_tests,
					(run_tests_series, run_non_send_tests_series),
					// #[cfg(not(test))]
					insert_final_outcome,
					log_incremental,
					log_final,
					exit_on_done,
				)
					.chain(),
			);
	}
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RunTests;
