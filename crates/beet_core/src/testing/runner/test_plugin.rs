//! Test runner plugin and schedule for beet.

use super::*;
use crate::prelude::*;
use bevy::ecs::schedule::ExecutorKind;
use bevy::time::TimePlugin;

/// Builds and runs the test app for the given owned tests.
fn run_tests_app(tests: Vec<TestDescAndFn>) {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AppExitPlugin, TestPlugin))
		.spawn_then((TestRunnerConfig::from_env(), tests_bundle(tests)))
		.run();
}

// On wasm the linker only calls `__wasm_call_ctors` from exported functions
// under "command-style linkage" heuristics; calling it explicitly guarantees
// `inventory`'s registration constructors have run before we collect. It is
// idempotent for inventory-generated constructors. See the inventory docs.
#[cfg(target_family = "wasm")]
unsafe extern "C" {
	fn __wasm_call_ctors();
}

/// Stable-Rust entry point: runs every [`BeetTestCase`] registered via
/// [`inventory`]. Invoked by the `beet_core::test_main!()` macro.
pub fn test_main() {
	#[cfg(target_family = "wasm")]
	unsafe {
		__wasm_call_ctors();
	}
	run_tests_app(inventory_tests());
}

/// Runs an explicit set of beet tests, cloning static descriptors.
///
/// Used by the `examples/runner.rs` demo. The nightly
/// `custom_test_frameworks` harness uses [`libtest_runner`] instead.
pub fn test_runner(tests: &[&TestDescAndFn]) {
	run_tests_app(tests.iter().map(|t| test_ext::clone_static(t)).collect());
}

/// Entry point for the nightly `custom_test_frameworks` test harness,
/// invoked via `#![test_runner(beet_core::libtest_runner)]`.
#[cfg(feature = "custom_test_frameworks")]
pub fn libtest_runner(tests: &[&test::TestDescAndFn]) {
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
