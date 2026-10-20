use crate::prelude::*;
use crate::testing::runner::MaybeAsync;
use crate::testing::runner::TestRunResult;
use crate::testing::runner::*;
use bevy::ecs::system::NonSendMarker;
use bevy::ecs::system::SystemParam;

/// Marks a test whose function has been called, holding one of its suite's
/// [`test_threads`](TestRunnerConfig::test_threads) slots until its
/// [`TestOutcome`] lands. Only a running test's timeout clock advances.
#[derive(Component)]
pub(crate) struct TestRunning;

#[track_caller]
pub(super) fn run_tests_series(
	mut commands: Commands,
	async_commands: AsyncCommands,
	slots: TestSlots,
	query: Populated<
		(Entity, &Test, &TestFunc, &ChildOf),
		(Without<TestRunning>, Without<TestOutcome>),
	>,
) -> Result {
	let mut free = slots.free();
	for (entity, test, func, parent) in query.iter() {
		let Some(suite) = free.claim(parent.parent()) else {
			continue;
		};
		let started = run_test(
			commands.reborrow(),
			&async_commands,
			suite.config,
			entity,
			test,
			move || func.run(),
		)?;
		suite.release_if(started == Started::Sync);
	}
	Ok(())
}

#[track_caller]
pub(super) fn run_non_send_tests_series(
	_: NonSendMarker,
	mut commands: Commands,
	async_commands: AsyncCommands,
	slots: TestSlots,
	mut query: Populated<
		(Entity, &Test, &mut NonSendTestFunc, &ChildOf),
		(Without<TestRunning>, Without<TestOutcome>),
	>,
) -> Result {
	let mut free = slots.free();
	for (entity, test, mut func, parent) in query.iter_mut() {
		let Some(suite) = free.claim(parent.parent()) else {
			continue;
		};
		commands.entity(entity).remove::<NonSendTestFunc>();
		let func = core::mem::replace(
			func.as_mut(),
			// unreachable because we remove the component immediately
			NonSendTestFunc::new(|| unreachable!("test func already taken")),
		);
		let started = run_test(
			commands.reborrow(),
			&async_commands,
			suite.config,
			entity,
			test,
			move || func.run(),
		)?;
		suite.release_if(started == Started::Sync);
	}
	Ok(())
}

/// Each suite's free [`test_threads`](TestRunnerConfig::test_threads) slots:
/// its cap less its tests still running.
#[derive(SystemParam)]
pub(super) struct TestSlots<'w, 's> {
	suites: Query<'w, 's, (Entity, &'static TestRunnerConfig)>,
	running: Query<
		'w,
		's,
		&'static ChildOf,
		(With<TestRunning>, Without<TestOutcome>),
	>,
}

impl TestSlots<'_, '_> {
	/// Snapshot the free slots, for one system's worth of starts.
	fn free(&self) -> FreeSlots<'_> {
		FreeSlots(
			self.suites
				.iter()
				.map(|(suite, config)| {
					let running = self
						.running
						.iter()
						.filter(|parent| parent.parent() == suite)
						.count();
					(suite, SuiteSlots {
						config,
						free: config.test_threads.saturating_sub(running),
					})
				})
				.collect(),
		)
	}
}

/// The free slots of every suite, claimed as tests start.
struct FreeSlots<'a>(HashMap<Entity, SuiteSlots<'a>>);

impl<'a> FreeSlots<'a> {
	/// Claim a slot in `suite` for the next test, `None` while it is full.
	fn claim(&mut self, suite: Entity) -> Option<&mut SuiteSlots<'a>> {
		self.0
			.get_mut(&suite)
			.filter(|slots| slots.free > 0)
			.map(|slots| {
				slots.free -= 1;
				slots
			})
	}
}

/// One suite's config and free slot count.
struct SuiteSlots<'a> {
	config: &'a TestRunnerConfig,
	free: usize,
}

impl SuiteSlots<'_> {
	/// Return the claimed slot when the test held it for no time at all, ie it
	/// ran synchronously to its outcome.
	fn release_if(&mut self, release: bool) {
		if release {
			self.free += 1;
		}
	}
}

/// How a test ran once started.
#[derive(PartialEq)]
enum Started {
	/// To its outcome, before returning.
	Sync,
	/// As a task, holding its slot until its outcome lands.
	Async,
}

#[track_caller]
fn run_test(
	mut commands: Commands,
	async_commands: &AsyncCommands,
	config: &TestRunnerConfig,
	entity: Entity,
	test: &Test,
	func: impl FnOnce() -> Result<(), String>,
) -> Result<Started> {
	// before the call, so a test that hangs the thread was already named
	if config.log_runs {
		log_case_runs(test, !config.no_color).xprint_display();
	}
	let should_panic = test.should_panic;
	let TestRunResult {
		maybe_async,
		params,
	} = super::try_run_async(func);

	// Always insert test params (they're always present now)
	commands.entity(entity).insert((params, TestRunning));

	match maybe_async {
		MaybeAsync::Sync(panic_result) => {
			let outcome =
				TestOutcome::from_panic_result(panic_result, should_panic);
			commands.entity(entity).insert(outcome);
			Started::Sync.xok()
		}
		MaybeAsync::Async(panic_result_fut) => {
			// a world task: the test future is `!Send`, so it cannot ride a
			// queued (entity-scoped) command, and a test entity never despawns.
			async_commands.run_local(async move |world| {
				let result = panic_result_fut.await;
				let outcome =
					TestOutcome::from_panic_result(result, should_panic);
				// Don't clobber a `TestOutcome` already set this frame (eg a
				// timeout); a test that finishes after timing out stays a timeout.
				world
					.entity(entity)
					.with(move |mut entity| {
						if !entity.contains::<TestOutcome>() {
							entity.insert(outcome);
						}
					})
					.await
					.ok();
			});
			Started::Async.xok()
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	async fn run_test(test: TestDescAndFn) -> TestOutcome {
		test_runner_ext::run(None, test).await
	}

	#[crate::test]
	async fn works_sync() {
		run_test(test_ext::new_auto(|| Ok(())))
			.await
			.xpect_eq(TestOutcome::Pass);
		run_test(test_ext::new_auto(|| Err("pizza".into())))
			.await
			.xpect_eq(
				TestFail::Err {
					message: "pizza".into(),
				}
				.into(),
			);
		run_test(test_ext::new_auto(|| panic!("expected")).with_should_panic())
			.await
			.xpect_eq(TestOutcome::Pass);
		run_test(test_ext::new_auto(|| Ok(())).with_should_panic())
			.await
			.xpect_eq(TestFail::ExpectedPanic { message: None }.into());
		run_test(
			test_ext::new_auto(|| panic!("boom"))
				.with_should_panic_message("boom"),
		)
		.await
		.xpect_eq(TestOutcome::Pass);
		run_test(
			test_ext::new_auto(|| Ok(())).with_should_panic_message("boom"),
		)
		.await
		.xpect_eq(
			TestFail::ExpectedPanic {
				message: Some("boom".into()),
			}
			.into(),
		);
		let line = line!() + 1;
		run_test(test_ext::new_auto(|| panic!("pizza")))
			.await
			.xpect_eq(
				TestFail::Panic {
					payload: Some("pizza".into()),
					location: Some(FileSpan::new_with_start(file!(), line, 39)),
				}
				.into(),
			);
	}

	/// At most `--test-threads` tests are in flight at once.
	#[crate::test]
	async fn caps_tests_in_flight() {
		use core::sync::atomic::AtomicUsize;
		use core::sync::atomic::Ordering;
		use std::sync::Arc;

		let running = Arc::new(AtomicUsize::new(0));
		let busiest = Arc::new(AtomicUsize::new(0));
		let tests = (0..6)
			.map(|_| {
				let (running, busiest) = (running.clone(), busiest.clone());
				test_ext::new_auto(move || {
					register_test(TestCaseParams::new(), async move {
						let now = running.fetch_add(1, Ordering::SeqCst) + 1;
						busiest.fetch_max(now, Ordering::SeqCst);
						for _ in 0..4 {
							async_ext::yield_now().await;
						}
						running.fetch_sub(1, Ordering::SeqCst);
						Ok::<(), String>(())
					});
					Ok(())
				})
			})
			.collect();
		test_runner_ext::run_all(Some("--test-threads=2"), tests)
			.await
			.xpect_eq(vec![TestOutcome::Pass; 6]);
		busiest.load(Ordering::SeqCst).xpect_eq(2);
	}

	#[crate::test]
	async fn works_async() {
		run_test(test_ext::new_auto(|| {
			register_test(TestCaseParams::new(), async {
				async_ext::yield_now().await;
				Ok::<(), String>(())
			});
			Ok(())
		}))
		.await
		.xpect_eq(TestOutcome::Pass);

		run_test(test_ext::new_auto(|| {
			register_test(TestCaseParams::new(), async {
				async_ext::yield_now().await;
				Err::<(), String>("pizza".into())
			});
			Ok(())
		}))
		.await
		.xpect_eq(
			TestFail::Err {
				message: "pizza".into(),
			}
			.into(),
		);

		run_test(
			test_ext::new_auto(|| {
				register_test(TestCaseParams::new(), async {
					async_ext::yield_now().await;
					panic!("expected");
					#[allow(unreachable_code)]
					Ok::<(), String>(())
				});
				Ok(())
			})
			.with_should_panic(),
		)
		.await
		.xpect_eq(TestOutcome::Pass);

		run_test(
			test_ext::new_auto(|| {
				register_test(TestCaseParams::new(), async {
					async_ext::yield_now().await;
					Ok::<(), String>(())
				});
				Ok(())
			})
			.with_should_panic(),
		)
		.await
		.xpect_eq(TestFail::ExpectedPanic { message: None }.into());

		run_test(
			test_ext::new_auto(|| {
				register_test(TestCaseParams::new(), async {
					async_ext::yield_now().await;
					panic!("boom");
					#[allow(unreachable_code)]
					Ok::<(), String>(())
				});
				Ok(())
			})
			.with_should_panic_message("boom"),
		)
		.await
		.xpect_eq(TestOutcome::Pass);

		run_test(
			test_ext::new_auto(|| {
				register_test(TestCaseParams::new(), async {
					async_ext::yield_now().await;
					Ok::<(), String>(())
				});
				Ok(())
			})
			.with_should_panic_message("boom"),
		)
		.await
		.xpect_eq(
			TestFail::ExpectedPanic {
				message: Some("boom".into()),
			}
			.into(),
		);

		let line = line!() + 4;
		run_test(test_ext::new_auto(|| {
			register_test(TestCaseParams::new(), async {
				async_ext::yield_now().await;
				panic!("pizza");
				#[allow(unreachable_code)]
				Ok::<(), String>(())
			});
			Ok(())
		}))
		.await
		.xpect_eq(
			TestFail::Panic {
				payload: Some("pizza".into()),
				location: Some(FileSpan::new_with_start(file!(), line, 16)),
			}
			.into(),
		);
	}

	#[crate::test]
	async fn unified_registration() {
		use crate::testing::runner::register_test;

		// Test unified registration with params
		run_test(test_ext::new_auto(|| {
			register_test(TestCaseParams::new().with_timeout_ms(5000), async {
				async_ext::yield_now().await;
				Ok::<(), String>(())
			});
			Ok(())
		}))
		.await
		.xpect_eq(TestOutcome::Pass);

		// Test unified registration handles failures
		run_test(test_ext::new_auto(|| {
			register_test(TestCaseParams::new(), async {
				async_ext::yield_now().await;
				Err::<(), String>("unified error".into())
			});
			Ok(())
		}))
		.await
		.xpect_eq(
			TestFail::Err {
				message: "unified error".into(),
			}
			.into(),
		);

		// Test unified registration with should_panic
		run_test(
			test_ext::new_auto(|| {
				register_test(TestCaseParams::new(), async {
					async_ext::yield_now().await;
					panic!("expected panic");
					#[allow(unreachable_code)]
					Ok::<(), String>(())
				});
				Ok(())
			})
			.with_should_panic(),
		)
		.await
		.xpect_eq(TestOutcome::Pass);
	}
}
