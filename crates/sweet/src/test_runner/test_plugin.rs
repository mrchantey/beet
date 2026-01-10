use super::*;
use beet_core::prelude::*;
use bevy::time::TimePlugin;


pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	use beet_net::prelude::*;

	App::new()
		.init_plugin::<JsRuntimePlugin>()
		.add_plugins((MinimalPlugins, TestPlugin))
		.spawn_then((
			Request::from_cli_args(CliArgs::parse_env()).unwrap_or_exit(),
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

		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<TimePlugin>()
			.insert_schedule_before(Update, RunTests)
			.add_systems(
				RunTests,
				(
					log_suite_running,
					filter_tests,
					log_case_running,
					(run_tests_series, run_non_send_tests_series),
					trigger_timeouts,
					insert_suite_outcome,
					log_case_outcomes,
					log_suite_outcome,
					exit_on_suite_outcome,
				)
					.chain(),
			);
	}
}


#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RunTests;
