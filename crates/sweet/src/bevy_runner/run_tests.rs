use crate::bevy_runner::MaybeAsync;
use crate::prelude::*;
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
				world.entity(entity).insert(outcome).await;
			});
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

		app.run_async().await;
		// app.run_loop();
		store.get().unwrap()
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
		run_test(test_ext::new_auto(|| panic!("pizza")))
			.await
			.xpect_eq(
				TestFail::Panic {
					payload: Some("pizza".into()),
					location: Some(FileSpan::new_with_start(
						file!(),
						line!() - 7,
						39,
					)),
				}
				.into(),
			);
	}

	// Async tests cannot be tested in nested apps on WASM because
	// async tasks require JS event loop ticks to progress, which don't
	// happen in a synchronous update loop. These tests work on native.
	// #[cfg(not(target_arch = "wasm32"))]
	#[sweet::test]
	async fn works_async() {
		use crate::bevy_runner::register_async_test;

		
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

		run_test(test_ext::new_auto(|| {
			register_async_test(async {
				async_ext::yield_now().await;
				async_ext::yield_now().await;
				panic!("pizza")
			});
			Ok(())
		}))
		.await
		.xpect_eq(
			TestFail::Panic {
				payload: Some("pizza".into()),
				location: Some(FileSpan::new_with_start(
					file!(),
					line!() - 10,
					16,
				)),
			}
			.into(),
		);
	}
}
