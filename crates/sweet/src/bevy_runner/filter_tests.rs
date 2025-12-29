use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;



/// Allow filtering tests by either named params or positional arguments,
/// so `test foobar.ts` is the same as `test --include foober.ts`
#[derive(Reflect)]
pub struct FilterParams {
	filter: GlobFilter,
	/// By default the glob filter will wrap
	/// all patterns in wildcards, so `*foo*` will match `/foo.rs`.
	/// Specify `--exact` to disable this, ensuring an exact match.
	exact: bool,
}

impl FilterParams {
	/// merge the positional args with params, and check for 'exact' specification
	fn parse(mut self, path_args: Option<&Vec<String>>) -> Self {
		if let Some(args) = path_args {
			self.filter = self.filter.extend_include(args);
		}
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
	route_query: RouteQuery,
	requests: Populated<(Entity, &RequestMeta, &Children), Added<RequestMeta>>,
	tests: Populated<(Entity, &Test), Added<Test>>,
) -> Result {
	for (entity, request, children) in requests {
		// TODO this is incorrect, action may be different entity
		let path_match = route_query.path_match(&ActionExchange {
			action: entity,
			exchange: entity,
		})?;
		let path_args = path_match.dyn_map.get_vec("include");

		let filter = request.params().parse::<FilterParams>()?.parse(path_args);



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
			PathPartial::new("*include?"),
			Request::from_cli_str(args).unwrap(),
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
		passes_filter("filter_tests.rs").xpect_true();
		passes_filter("foobar").xpect_false();
		passes_filter("--include foobar").xpect_false();
		passes_filter("--include *filter_tests.rs").xpect_true();
	}
}
