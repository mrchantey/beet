use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;


/// Added to the [`Request`] entity once all tests have completed
#[derive(Component)]
pub struct SuiteOutcome {
	num_pass: usize,
	num_skip: usize,
	num_fail: usize,
}

impl SuiteOutcome {
	/// number of tests that passed
	pub fn num_pass(&self) -> usize { self.num_pass }
	/// number of tests that were skipped
	pub fn num_skip(&self) -> usize { self.num_skip }
	/// number of tests that failed
	pub fn num_fail(&self) -> usize { self.num_fail }
	/// number of tests that were run (not skipped)
	pub fn num_ran(&self) -> usize { self.num_pass + self.num_fail }
	/// total number of tests, including those skipped
	pub fn num_total(&self) -> usize {
		self.num_pass + self.num_skip + self.num_fail
	}

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

/// Insert final when no tests to run
pub fn insert_suite_outcome(
	mut commands: Commands,
	requests: Populated<
		(Entity, &RequestMeta, Option<&Children>),
		Without<SuiteOutcome>,
	>,
	// listener query, running this system on
	// either
	// - added request (in case none to run)
	// - added outcome (in case all done)
	_listener: Populated<(), Or<(Added<RequestMeta>, Added<TestOutcome>)>>,
	all_finished: Query<(&Test, &TestOutcome)>,
	still_running: Query<(), (With<Test>, Without<TestOutcome>)>,
) {
	for (entity, _req, children) in requests {
		let Some(children) = children else {
			commands.entity(entity).insert(SuiteOutcome::new(&[]));
			continue;
		};

		let still_running = children
			.iter()
			.filter_map(|child| still_running.get(child).ok())
			.count();
		if still_running > 0 {
			continue;
		}
		let all_finished = children
			.iter()
			.filter_map(|child| all_finished.get(child).ok())
			.collect::<Vec<_>>();
		commands
			.entity(entity)
			.insert(SuiteOutcome::new(&all_finished));
	}
}
