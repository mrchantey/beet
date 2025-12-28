use crate::prelude::*;
use beet_core::prelude::*;
// use beet_flow::prelude::*;
use beet_net::prelude::*;


#[derive(Reflect)]
pub struct FilterParams {
	filter: GlobFilter,
	/// By default the glob filter will wrap
	/// all patterns in wildcards, so `*foo*` will match `/foo.rs`.
	/// Specify `--exact` to disable this, ensuring an exact match.
	exact: bool,
}

impl FilterParams {
	fn try_wrap(mut self) -> Self {
		if !self.exact {
			self.filter.wrap_all_with_wildcard();
		}
		self
	}

	fn passes(&self, test: &Test) -> bool {
		self.filter.passes(test.name.to_string())
			|| self.filter.passes(test.source_file)
	}
}


pub fn filter_tests(
	mut commands: Commands,
	requests: Populated<(&RequestMeta, &Children), Added<RequestMeta>>,
	tests: Populated<(Entity, &Test), Added<Test>>,
) -> Result {
	for (request, children) in requests {
		let filter = request.params().parse::<FilterParams>()?.try_wrap();

		for (entity, _test) in children
			.iter()
			.filter_map(|child| tests.get(child).ok())
			.filter(|(_, test)| !filter.passes(test))
		{
			commands.entity(entity).insert(Disabled);
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
			Request::from_cli_args(CliArgs::parse(args)).unwrap(),
			tests_bundle(vec![test_ext::new_auto(|| Ok(()))]),
		));
		world.update();
		world
			.query_filtered_once::<&Test, Without<Disabled>>()
			.len() == 1
	}

	#[test]
	fn works() {
		passes_filter("").xpect_true();
		passes_filter("--include foobar").xpect_false();
		passes_filter("--include *filter_tests.rs").xpect_true();
	}
}
