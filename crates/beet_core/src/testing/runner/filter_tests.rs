//! Test filtering based on glob patterns.

use crate::prelude::*;
use crate::testing::runner::*;


/// Filters tests based on request parameters, marking non-matching tests as skipped.
pub fn filter_tests(
	mut commands: Commands,
	requests: Populated<
		(&TestRunnerConfig, &Children),
		Added<TestRunnerConfig>,
	>,
	tests: Populated<(Entity, &Test, Option<&TestOutcome>), Added<Test>>,
) -> Result {
	for (config, children) in requests {
		for (entity, test, outcome) in
			children.iter().filter_map(|child| tests.get(child).ok())
		{
			// Handle --ignored and --include-ignored flags
			if let Some(TestOutcome::Skip(TestSkip::Ignore(_))) = outcome {
				// Test is marked as ignored
				if config.ignored {
					// --ignored: run ignored tests, remove the skip outcome
					commands.entity(entity).remove::<TestOutcome>();
				} else if !config.include_ignored {
					// Default: keep the skip (already set by try_skip)
					continue;
				} else {
					// --include-ignored: run both, remove the skip outcome
					commands.entity(entity).remove::<TestOutcome>();
				}
			} else if config.ignored {
				// --ignored: skip non-ignored tests
				commands
					.entity(entity)
					.insert(TestOutcome::Skip(TestSkip::FailedFilter));
				continue;
			}

			// Apply glob filter
			if !config.passes_filter(test) {
				commands
					.entity(entity)
					.insert(TestOutcome::Skip(TestSkip::FailedFilter));
			}
		}
	}
	Ok(())
}


// fn test_passes_request
#[cfg(test)]
mod tests {
	use super::*;

	fn passes_filter(args: &str) -> bool {
		let mut world = TestPlugin::world();
		world.spawn((
			TestRunnerConfig::from_cli_str(args),
			tests_bundle(vec![test_ext::new_auto(|| Ok(()))]),
		));
		world.update_local();
		world.query_once::<&TestOutcome>()[0] == &TestOutcome::Pass
	}

	#[test]
	fn works() {
		passes_filter("--quiet").xpect_true();
		passes_filter("filter_tests.rs --quiet").xpect_true();
		passes_filter("foobar --quiet").xpect_false();
		passes_filter("--quiet --include foobar").xpect_false();
		passes_filter("--quiet --include *filter_tests.rs").xpect_true();
	}

	#[test]
	fn ignored_flags() {
		// Default: ignored tests are skipped
		let mut world = TestPlugin::world();
		let mut ignored_test = test_ext::new_auto(|| Ok(()));
		ignored_test.desc.ignore = true;
		ignored_test.desc.ignore_message = Some("test is ignored");
		world.spawn((
			TestRunnerConfig::from_cli_str("--quiet"),
			tests_bundle(vec![ignored_test]),
		));
		world.update_local();
		let outcome = world.query_once::<&TestOutcome>()[0];
		matches!(outcome, TestOutcome::Skip(TestSkip::Ignore(_))).xpect_true();

		// --include-ignored: ignored tests should run
		let mut world = TestPlugin::world();
		let mut ignored_test = test_ext::new_auto(|| Ok(()));
		ignored_test.desc.ignore = true;
		ignored_test.desc.ignore_message = Some("test is ignored");
		world.spawn((
			TestRunnerConfig::from_cli_str("--quiet --include-ignored"),
			tests_bundle(vec![ignored_test]),
		));
		world.update_local();
		let outcome = world.query_once::<&TestOutcome>()[0];
		(outcome == &TestOutcome::Pass).xpect_true();

		// --ignored: only ignored tests run, non-ignored are skipped
		let mut world = TestPlugin::world();
		let mut ignored_test = test_ext::new_auto(|| Ok(()));
		ignored_test.desc.ignore = true;
		ignored_test.desc.ignore_message = Some("test is ignored");
		let normal_test = test_ext::new_auto(|| Ok(()));
		world.spawn((
			TestRunnerConfig::from_cli_str("--quiet --ignored"),
			tests_bundle(vec![ignored_test, normal_test]),
		));
		world.update_local();
		let outcomes = world.query_once::<&TestOutcome>();
		// First test (ignored) should pass
		(outcomes[0] == &TestOutcome::Pass).xpect_true();
		// Second test (normal) should be skipped
		matches!(outcomes[1], TestOutcome::Skip(TestSkip::FailedFilter))
			.xpect_true();
	}
}
