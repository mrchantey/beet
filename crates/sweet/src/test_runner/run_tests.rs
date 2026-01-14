use crate::prelude::*;
use crate::test_runner::MaybeAsync;
use beet_core::prelude::*;
use bevy::ecs::system::NonSendMarker;



#[track_caller]
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
			test.should_panic,
			move || func.run(),
		)?;
	}
	Ok(())
}




#[track_caller]
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
			test.should_panic,
			move || func.run(),
		)?;
	}
	Ok(())
}


#[track_caller]
fn run_test(
	mut commands: Commands,
	mut async_commands: AsyncCommands,
	entity: Entity,
	should_panic: test::ShouldPanic,
	func: impl FnOnce() -> Result<(), String>,
) -> Result {
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

	async fn run_test(test: TestDescAndFn) -> TestOutcome {
		test_runner_ext::run(None, test).await
	}

	#[sweet::test]
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

	#[sweet::test]
	async fn works_async() {
		use crate::test_runner::register_async_test;


		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				Ok(())
			});
			Ok(())
		}))
		.await
		.xpect_eq(TestOutcome::Pass);

		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				Err("pizza".into())
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
				register_async_test(async {
					async_ext::yield_now().await;
					panic!("expected")
				});
				Ok(())
			})
			.with_should_panic(),
		)
		.await
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
		.await
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
		.await
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
		.await
		.xpect_eq(
			TestFail::ExpectedPanic {
				message: Some("boom".into()),
			}
			.into(),
		);

		let line = line!() + 4;
		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				panic!("pizza")
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
}
