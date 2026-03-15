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
	tests: Populated<(Entity, &Test), Added<Test>>,
) -> Result {
	for (config, children) in requests {
		for (entity, _test) in children
			.iter()
			.filter_map(|child| tests.get(child).ok())
			.filter(|(_, test)| !config.passes_filter(test))
		{
			commands
				.entity(entity)
				.insert(TestOutcome::Skip(TestSkip::FailedFilter));
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
}
