//! Test filtering based on glob patterns.

use crate::prelude::*;
use crate::testing::runner::*;



/// Allow filtering tests by either named params or positional arguments,
/// so `test foobar.ts` is the same as `test --include foober.ts`
#[derive(Debug, Clone, Reflect, Component, Default)]
#[reflect(Default)]
pub struct FilterParams {
	/// Glob pattern filter for test selection.
	pub filter: GlobFilter,
	/// By default the glob filter will wrap
	/// all patterns in wildcards, so `*foo*` will match `/foo.rs`.
	/// Specify `--exact` to disable this, ensuring an exact match.
	exact: bool,
}


impl FilterParams {
	fn new(args: &TestRunnerArgs) -> Result<Self> {
		let mut this = args.params().parse_reflect::<FilterParams>()?;
		// extend include by positional args
		this.filter = this.filter.extend_include(args.path());
		// check for 'exact' specification
		if !this.exact {
			this.filter.wrap_all_with_wildcard();
		}
		this.xok()
	}

	fn passes(&self, test: &Test) -> bool {
		self.filter.passes(test.name.to_string())
			|| self.filter.passes(test.source_file)
	}
}

/// Filters tests based on request parameters, marking non-matching tests as skipped.
pub fn filter_tests(
	mut commands: Commands,
	requests: Populated<(&TestRunnerArgs, &Children), Added<TestRunnerArgs>>,
	tests: Populated<(Entity, &Test), Added<Test>>,
) -> Result {
	for (args, children) in requests {
		// we dont use Extractor because this has extra extractor steps
		let filter = FilterParams::new(args)?;

		for (entity, _test) in children
			.iter()
			.filter_map(|child| tests.get(child).ok())
			.filter(|(_, test)| !filter.passes(test))
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
			TestRunnerArgs::from_cli_str(args),
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
