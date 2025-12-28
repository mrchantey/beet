use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


/// Added to the [`Request`] entity once all tests have completed
#[derive(Component)]
pub struct FinalOutcome {
	num_pass: usize,
	num_skip: usize,
	num_fail: usize,
}

impl FinalOutcome {
	pub fn num_pass(&self) -> usize { self.num_pass }
	pub fn num_skip(&self) -> usize { self.num_skip }
	pub fn num_fail(&self) -> usize { self.num_fail }


	pub fn new(tests: &[(&Test, &TestOutcome)]) -> Self {
		let mut num_pass = 0;
		let mut num_skip = 0;
		let mut num_fail = 0;
		for (_test, outcome) in tests {
			match outcome {
				TestOutcome::Pass => num_pass += 1,
				TestOutcome::Skip(_) => num_skip += 1,
				TestOutcome::Fail(_) => num_fail += 1,
			}
		}
		Self {
			num_pass,
			num_skip,
			num_fail,
		}
	}
}

/// Exits when all tests have finished
pub fn insert_final_outcome(
	mut commands: Commands,
	requests: Populated<(Entity, &RequestMeta, &Children)>,
	// run on change
	_just_finished: Populated<(), Added<TestOutcome>>,
	all_finished: Query<(&Test, &TestOutcome)>,
	all_tests: Query<(), With<Test>>,
) {
	for (entity, _req, children) in requests {
		let all_finished = children
			.iter()
			.filter_map(|child| all_finished.get(child).ok())
			.collect::<Vec<_>>();
		let num_tests = children
			.iter()
			.filter_map(|child| all_tests.get(child).ok())
			.count();
		if all_finished.len() != num_tests {
			// not done yet
			continue;
		}
		let outcome = FinalOutcome::new(&all_finished);
		commands.entity(entity).insert(outcome);
	}
}
