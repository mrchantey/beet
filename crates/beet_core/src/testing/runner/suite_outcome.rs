use crate::prelude::*;
use crate::testing::runner::*;


/// Added to the [`TestRunnerConfig`] entity once all tests have completed
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

	/// Creates a new outcome by tallying pass/skip/fail counts from the given tests.
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
	// matches runners even if the config was not just added
	runners: Populated<
		(Entity, Option<&Children>),
		(With<TestRunnerConfig>, Without<SuiteOutcome>),
	>,
	// listener query, running this system on a match
	_listener: Populated<
		(),
		Or<(
			// config added, but may not have children if no tests to run
			Added<TestRunnerConfig>,
			// individual test finished
			Added<TestOutcome>,
		)>,
	>,
	all_finished: Query<(&Test, &TestOutcome)>,
	still_running: Query<(), (With<Test>, Without<TestOutcome>)>,
) {
	for (entity, children) in runners {
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

pub(crate) fn trigger_timeouts(
	mut commands: Commands,
	time: Res<Time>,
	configs: Query<&TestRunnerConfig>,
	mut query: Populated<
		(Entity, &mut Test, &ChildOf, Option<&TestCaseParams>),
		Without<TestOutcome>,
	>,
) -> Result {
	for (entity, mut test, parent, test_params) in query.iter_mut() {
		let config = configs.get(parent.0)?;
		// Check per-test timeout first, then fall back to suite timeout
		let timeout = if let Some(test_params) = test_params {
			if let Some(timeout) = test_params.timeout {
				timeout
			} else {
				config.timeout()
			}
		} else {
			config.timeout()
		};

		test.tick(time.delta());
		let elapsed = test.elapsed();
		if elapsed >= timeout {
			commands
				.entity(entity)
				.insert(TestOutcome::Fail(TestFail::Timeout { elapsed }));
		}
	}
	Ok(())
}


/// These tests require the custom test runner (ECS-driven async with
/// `test_runner_ext::run`) which is only available on nightly.
#[cfg(test)]
#[cfg(feature = "custom_test_framework")]
mod tests {
	use super::*;
	use crate::testing::runner::TestDescAndFn;
	use crate::testing::runner::TestFn;


	async fn did_timeout(test: TestDescAndFn) -> bool {
		test_runner_ext::run(Some("--timeout-ms=10"), test)
			.await
			.as_fail()
			.unwrap()
			.is_timeout()
	}

	fn loop_sync() {
		let elapsed = Instant::now();
		loop {
			if elapsed.elapsed().as_millis() > 100 {
				panic!("should timeout");
			}
		}
	}

	#[crate::test]
	async fn timeout_non_send_sync() {
		did_timeout(TestDescAndFn {
			desc: test_ext::new_auto_desc(),
			testfn: TestFn::DynTestFn(Box::new(|| loop_sync().xok())),
		})
		.await
		.xpect_false();
	}

	// Note: We cannot timeout pure synchronous Send+Sync tests (StaticTestFn)
	// because they might call `register_test`, which uses thread-local
	// storage. If we spawn the test in a separate thread for timeout enforcement,
	// the async test registration happens in the wrong thread and is lost.
	// Therefore, we rely on `trigger_timeouts` for async tests only.

	#[crate::test]
	async fn timeout_async() {
		did_timeout(test_ext::new_auto(|| {
			register_test(TestCaseParams::new(), async {
				time_ext::sleep_millis(100).await;
				unreachable!("should timeout")
			});
			Ok(())
		}))
		.await
		.xpect_true();
	}

	#[crate::test]
	async fn per_test_timeout_overrides_suite() {
		// Suite timeout is 10ms, but per-test timeout is 1000ms
		// Test sleeps for 50ms, so suite would timeout but per-test won't
		let test = test_ext::new_auto(|| {
			register_test(TestCaseParams::new().with_timeout_ms(1000), async {
				time_ext::sleep_millis(50).await;
				Ok(())
			});
			Ok(())
		});

		test_runner_ext::run(Some("--timeout-ms=10"), test)
			.await
			.xpect_eq(TestOutcome::Pass);
	}

	#[crate::test]
	async fn per_test_timeout_enforced() {
		// Per-test timeout is 10ms, test sleeps for 100ms
		let test = test_ext::new_auto(|| {
			register_test(TestCaseParams::new().with_timeout_ms(10), async {
				time_ext::sleep_millis(100).await;
				unreachable!("should timeout")
			});
			Ok(())
		});

		test_runner_ext::run(Some("--timeout-ms=5000"), test)
			.await
			.as_fail()
			.unwrap()
			.is_timeout()
			.xpect_true();
	}

	#[crate::test]
	async fn macro_timeout_enforced() {
		// Test that per-test timeout from macro attribute works
		let test = test_ext::new_auto(|| {
			register_test(TestCaseParams::new().with_timeout_ms(10), async {
				time_ext::sleep_millis(100).await;
				unreachable!("should timeout")
			});
			Ok(())
		});

		test_runner_ext::run(Some("--timeout-ms=5000"), test)
			.await
			.as_fail()
			.unwrap()
			.is_timeout()
			.xpect_true();
	}

	#[crate::test]
	async fn macro_timeout_not_reached() {
		// Test that per-test timeout allows test to complete if under limit
		let test = test_ext::new_auto(|| {
			register_test(TestCaseParams::new().with_timeout_ms(5000), async {
				time_ext::sleep_millis(10).await;
				Ok(())
			});
			Ok(())
		});

		test_runner_ext::run(Some("--timeout-ms=10"), test)
			.await
			.xpect_eq(TestOutcome::Pass);
	}

	#[crate::test]
	async fn macro_timeout_sync() {
		// Test that per-test timeout works with sync wrapper for async test
		let test = test_ext::new_auto(|| {
			register_test(TestCaseParams::new().with_timeout_ms(5000), async {
				time_ext::sleep_millis(10).await;
				Ok(())
			});
			Ok(())
		});

		test_runner_ext::run(Some("--timeout-ms=10"), test)
			.await
			.xpect_eq(TestOutcome::Pass);
	}
}
