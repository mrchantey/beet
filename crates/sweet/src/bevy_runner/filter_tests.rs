use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Serialize, Deserialize)]
pub struct FilterParams {
	filter: GlobFilter,
}


pub fn filter_tests(
	mut commands: Commands,
	requests: Populated<(&RequestMeta, &Children), Added<RequestMeta>>,
	tests: Populated<(Entity, &Test), Added<Test>>,
) -> Result {
	for (request, children) in requests {
		
		// let filter = 
		
		
		let passes = |_test: &Test| false;

		for (entity, _test) in children
			.iter()
			.filter_map(|child| tests.get(child).ok())
			.filter(|(_, test)| !passes(test))
		{
			commands.entity(entity).insert(Disabled);
		}
	}
	Ok(())
}


// fn test_passes_request
