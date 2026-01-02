use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;



#[derive(Debug, Clone, Reflect, Component)]
pub struct SuiteParams {
	/// Timeout per test in the suite
	timeout: Duration,
}

impl Default for SuiteParams {
	fn default() -> Self {
		Self {
			timeout: Duration::from_secs(5),
		}
	}
}

impl RequestMetaExtractor for SuiteParams {
	fn extract(request: &RequestMeta) -> Result<Self> {
		request.params().parse_reflect()
	}
}


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

pub fn trigger_timeouts(
	mut commands: Commands,
	time: Res<Time>,
	mut params: Extractor<SuiteParams>,
	mut query: Populated<(Entity, &mut Test, &ChildOf), Without<TestOutcome>>,
) -> Result {
	for (entity, mut test, parent) in query.iter_mut() {
		let params = params.get(parent.0)?;
		test.tick(time.delta());
		let elapsed = test.elapsed();
		if elapsed >= params.timeout {
			commands
				.entity(entity)
				.insert(TestOutcome::Fail(TestFail::Timeout { elapsed }));
		}
	}
	Ok(())
}


#[cfg(test)]
mod tests {
	use super::*;
	use beet_net::prelude::Request;
	use test::TestDescAndFn;

	async fn run_test(test: TestDescAndFn) -> TestOutcome {
		let mut app = App::new().with_plugins((
			// ensure app exits even with update loop
			MinimalPlugins,
			TestPlugin,
		));

		app.world_mut().spawn((
			Request::from_cli_str("--quiet").unwrap(),
			tests_bundle(vec![test]),
		));
		let store = Store::new(None);

		app.add_observer(
			move |ev: On<Insert, TestOutcome>,
			      outcomes: Query<&TestOutcome>| {
				store.set(Some(outcomes.get(ev.entity).unwrap().clone()));
			},
		);
		// app.init();
		// advance time past timeout
		// app.update_with_secs(10);
		app.run_async().await;
		store.get().unwrap()
	}

	#[crate::test]
	async fn timeout() {
		panic!("here we are");
		// run_test(test_ext::new_auto(|| {
		// 	register_async_test(async {
		// 		// time_ext::sleep_millis(15_000).await;
		// 		panic!("pizza")
		// 	});
		// 	Ok(())
		// }))
		// .await
		// .as_fail()
		// .unwrap()
		// .is_timeout()
		// .xpect_false();
	}
}
