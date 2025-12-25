use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


pub fn filter_tests(
	// mut commands: Commands,
	query: Populated<(Entity, &Test), Added<Test>>,
	requests: AgentQuery<&Request>,
) -> Result {
	for (action, _test) in query.iter() {
		let _request = requests.get(action)?;
		// TODO generic request type
	}
	Ok(())
}
