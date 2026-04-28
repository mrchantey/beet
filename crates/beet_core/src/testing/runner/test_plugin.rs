//! Test runner plugin and schedule for beet.

use super::*;
use crate::prelude::*;
use bevy::ecs::schedule::ExecutorKind;
use bevy::time::TimePlugin;

/// Entry point for the custom test runner, invoked by the test harness.
pub fn test_runner(tests: &[&test::TestDescAndFn]) {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AppExitPlugin, TestPlugin))
		.spawn_then((
			TestRunnerConfig::from_env(),
			tests_bundle_borrowed(tests),
		))
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
			// Force single-threaded execution so `spawn_local` in `run_tests_series`
			// always lands on the main thread's local executor.
			// With `bevy_multithreaded`, the default is `MultiThreaded`, which
			// can dispatch systems to worker threads whose thread-local executors
			// are not ticked by `tick_global_task_pools_on_main_thread`, causing
			// async test tasks to never complete.
			//
			// We could alternatively just mark run_tests_series with NonSendMarker,
			// but thats more error-prone
			.edit_schedule(RunTests, |schedule| {
				schedule.set_executor_kind(ExecutorKind::SingleThreaded);
			})
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
					log_file_outcomes,
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
