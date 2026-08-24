//! Test runner plugin and schedule for beet.

use super::*;
use crate::prelude::*;
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::time::TimePlugin;

/// Builds the test app for the given owned tests.
///
/// std-shaped: `MinimalPlugins`' run-once-and-exit plus [`AppExitPlugin`]'s
/// `process::exit`. The bare-metal build assembles its own app + runner +
/// `AppExit`-to-semihosting consumer instead, reusing [`TestPlugin`] and
/// [`tests_bundle`].
#[cfg(feature = "testing")]
fn tests_app(tests: Vec<TestDescAndFn>) -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AppExitPlugin, TestPlugin))
		.spawn((TestRunnerConfig::from_env(), tests_bundle(tests)));
	app
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
///
/// On wasm this is a no-op: `main` runs during module `init()`, and a wasm
/// suite instead boots through the exported [`test_start`], the same
/// awaited-async-start contract an app binary uses, so tests can await real
/// timers, fetches and `postMessage` while the host holds the page open.
#[cfg(feature = "testing")]
pub fn test_main() {
	#[cfg(not(target_arch = "wasm32"))]
	tests_app(inventory_tests()).run();
}

/// The wasm suite entry, emitted as an exported async `test_start` by the
/// `test_main!` macro and awaited by the host (deno.ts, browser.js) exactly
/// like an app binary's `start`: run the suite via [`App::run_async`], return
/// the exit code. The `test_` prefix avoids colliding with a `start` the lib
/// under test may itself export.
#[cfg(all(feature = "testing", target_arch = "wasm32"))]
pub async fn test_start() -> i32 {
	unsafe {
		__wasm_call_ctors();
	}
	tests_app(inventory_tests()).run_async().await.exit_code()
}

/// Runs an explicit set of beet tests, cloning static descriptors.
///
/// Used by the `examples/runner.rs` demo. The nightly
/// `custom_test_frameworks` harness uses [`libtest_runner`] instead.
#[cfg(feature = "testing")]
pub fn test_runner(tests: &[&TestDescAndFn]) {
	tests_app(tests.iter().map(|t| test_ext::clone_static(t)).collect()).run();
}

/// Entry point for the nightly `custom_test_frameworks` test harness,
/// invoked via `#![test_runner(beet_core::libtest_runner)]`. Native only in
/// practice: wasm targets are `harness = false` everywhere, booting through
/// [`test_start`] instead.
#[cfg(feature = "custom_test_frameworks")]
pub fn libtest_runner(tests: &[&test::TestDescAndFn]) {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AppExitPlugin, TestPlugin))
		.spawn((TestRunnerConfig::from_env(), tests_bundle_borrowed(tests)));
	app.run();
}

/// Bevy plugin that sets up the test runner infrastructure.
#[derive(Default)]
pub(crate) struct TestPlugin;

impl Plugin for TestPlugin {
	fn build(&self, app: &mut App) {
		#[cfg(target_arch = "wasm32")]
		console_error_panic_hook::set_once();

		// tests read credentials and paths from `.env`, so load it here rather
		// than relying on the caller (`just`, a js host) to have done it.
		env_ext::load_dotenv().ok();

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
				schedule.set_executor(SingleThreadedExecutor::new());
			})
			// Timeouts are checked in PreUpdate *before* the async bridge sync
			// point applies test-completion inserts, so a test that finishes
			// right at its deadline is still reported as a timeout rather than
			// racing its own completion.
			.add_systems(
				PreUpdate,
				trigger_timeouts
					.before(async_world_sync_point::<BeetAsyncSyncPoint>),
			)
			.add_systems(
				RunTests,
				(
					log_suite_running,
					filter_tests,
					log_case_running,
					(run_tests_series, run_non_send_tests_series),
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
pub(crate) struct RunTests;
