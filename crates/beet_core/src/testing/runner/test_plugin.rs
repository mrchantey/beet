//! Test runner plugin and schedule for beet.

use super::*;
use crate::prelude::*;
use bevy::time::TimePlugin;

/// Entry point for the nightly custom test runner, invoked by the test harness.
///
/// Only available with the `custom_test_framework` feature.
#[cfg(feature = "custom_test_framework")]
pub fn test_runner_nightly(tests: &[&test::TestDescAndFn]) {
	let beet_tests: Vec<TestDescAndFn> =
		tests.iter().map(|t| from_nightly_ref(t)).collect();
	run_test_app(beet_tests);
}

/// Entry point for the stable test runner, collects tests from `inventory`.
pub fn test_runner() {
	let tests = collect_inventory_tests();
	run_test_app(tests);
}

/// Shared implementation that launches the Bevy test app with the given tests.
fn run_test_app(tests: Vec<TestDescAndFn>) {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AppExitPlugin, TestPlugin))
		.spawn_then((TestRunnerConfig::from_env(), tests_bundle(tests)))
		.run();
}


/// Bevy plugin that sets up the test runner infrastructure.
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


/// Schedule that runs test execution systems.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct RunTests;
