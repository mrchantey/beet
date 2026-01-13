use crate::prelude::*;
use crate::test_runner::MaybeAsync;
use beet_core::prelude::*;
use bevy::ecs::system::NonSendMarker;



pub(super) fn run_tests_series(
	mut commands: Commands,
	mut async_commands: AsyncCommands,

	query: Populated<
		(Entity, &Test, &TestFunc),
		(Added<TestFunc>, Without<TestOutcome>),
	>,
) -> Result {
	for (entity, test, func) in query.iter() {
		run_test(
			commands.reborrow(),
			async_commands.reborrow(),
			entity,
			test,
			move || func.run(),
		)?;
	}
	Ok(())
}


pub(super) fn run_non_send_tests_series(
	_: NonSendMarker,
	mut commands: Commands,
	mut async_commands: AsyncCommands,
	mut query: Populated<
		(Entity, &Test, &mut NonSendTestFunc),
		(Added<NonSendTestFunc>, Without<TestOutcome>),
	>,
) -> Result {
	for (entity, test, mut func) in query.iter_mut() {
		commands.entity(entity).remove::<NonSendTestFunc>();
		let func = std::mem::replace(
			func.as_mut(),
			// unreachable because we remove the component immediately
			NonSendTestFunc::new(|| unreachable!("test func already taken")),
		);
		run_test(
			commands.reborrow(),
			async_commands.reborrow(),
			entity,
			test,
			#[track_caller]
			move || func.run(),
		)?;
	}
	Ok(())
}


fn run_test(
	mut commands: Commands,
	mut async_commands: AsyncCommands,
	entity: Entity,
	test: &Test,
	func: impl FnOnce() -> Result<(), String>,
) -> Result {
	let should_panic = test.should_panic;
	match super::try_run_async(func) {
		MaybeAsync::Sync(panic_result) => {
			let outcome =
				TestOutcome::from_panic_result(panic_result, should_panic);
			commands.entity(entity).insert(outcome);
		}
		MaybeAsync::Async(panic_result_fut) => {
			async_commands.run_local(async move |world| {
				let result = panic_result_fut.await;
				let outcome =
					TestOutcome::from_panic_result(result, should_panic);
				world.entity(entity).insert(outcome);
			});
		}
	}


	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use test::TestDescAndFn;

	fn run_test(test: TestDescAndFn) -> TestOutcome {
		test_runner_ext::run(None, test)
	}

	#[sweet::test]
	fn works_sync() {
		run_test(test_ext::new_auto(|| Ok(()))).xpect_eq(TestOutcome::Pass);
		run_test(test_ext::new_auto(|| Err("pizza".into()))).xpect_eq(
			TestFail::Err {
				message: "pizza".into(),
			}
			.into(),
		);
		run_test(test_ext::new_auto(|| panic!("expected")).with_should_panic())
			.xpect_eq(TestOutcome::Pass);
		run_test(test_ext::new_auto(|| Ok(())).with_should_panic())
			.xpect_eq(TestFail::ExpectedPanic { message: None }.into());
		run_test(
			test_ext::new_auto(|| panic!("boom"))
				.with_should_panic_message("boom"),
		)
		.xpect_eq(TestOutcome::Pass);
		run_test(
			test_ext::new_auto(|| Ok(())).with_should_panic_message("boom"),
		)
		.xpect_eq(
			TestFail::ExpectedPanic {
				message: Some("boom".into()),
			}
			.into(),
		);
		let line = line!() + 1;
		run_test(test_ext::new_auto(|| panic!("pizza"))).xpect_eq(
			TestFail::Panic {
				payload: Some("pizza".into()),
				location: Some(FileSpan::new_with_start(file!(), line, 39)),
			}
			.into(),
		);
	}

	#[sweet::test]
	fn works_async() {
		use crate::test_runner::register_async_test;


		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				Ok(())
			});
			Ok(())
		}))
		.xpect_eq(TestOutcome::Pass);

		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				Err("pizza".into())
			});
			Ok(())
		}))
		.xpect_eq(
			TestFail::Err {
				message: "pizza".into(),
			}
			.into(),
		);

		run_test(
			test_ext::new_auto(|| {
				register_async_test(async {
					async_ext::yield_now().await;
					panic!("expected")
				});
				Ok(())
			})
			.with_should_panic(),
		)
		.xpect_eq(TestOutcome::Pass);

		run_test(
			test_ext::new_auto(|| {
				register_async_test(async {
					async_ext::yield_now().await;
					Ok(())
				});
				Ok(())
			})
			.with_should_panic(),
		)
		.xpect_eq(TestFail::ExpectedPanic { message: None }.into());

		run_test(
			test_ext::new_auto(|| {
				register_async_test(async {
					async_ext::yield_now().await;
					panic!("boom")
				});
				Ok(())
			})
			.with_should_panic_message("boom"),
		)
		.xpect_eq(TestOutcome::Pass);

		run_test(
			test_ext::new_auto(|| {
				register_async_test(async {
					async_ext::yield_now().await;
					Ok(())
				});
				Ok(())
			})
			.with_should_panic_message("boom"),
		)
		.xpect_eq(
			TestFail::ExpectedPanic {
				message: Some("boom".into()),
			}
			.into(),
		);

		let line = line!() + 5;
		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				async_ext::yield_now().await;
				panic!("pizza")
			});
			Ok(())
		}))
		.xpect_eq(
			TestFail::Panic {
				payload: Some("pizza".into()),
				location: Some(FileSpan::new_with_start(file!(), line, 16)),
			}
			.into(),
		);
	}
}
