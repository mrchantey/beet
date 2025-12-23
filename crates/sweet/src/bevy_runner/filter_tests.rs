use crate::prelude::*;
use beet_core::prelude::*;


pub fn filter_tests(
	// mut commands: Commands,
	query: Populated<(Entity, &Test), Added<Test>>,
) -> Result {
	for _test in query.iter() {}
	Ok(())
}
