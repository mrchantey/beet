use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::NonSendMarker;

/// the error message
#[derive(Debug, Clone, PartialEq, Eq, Component)]
#[component(storage = "SparseSet")]
pub enum TestOutcome {
	/// The test either returned ok, or was expected to panic
	Pass,
	/// The test returned an [`Err(String)`]
	Err { message: String },
	/// The test did not panic but was expected to
	ExpectedPanic { message: Option<String> },
	/// The test panicked
	Panic {
		/// The payload downcast from the `Box<dyn Any>`
		/// panic payload, or 'opaque payload'
		payload: Option<String>,
		/// The location of the panic if available
		location: Option<FileSpan>,
	},
}

pub(super) fn run_tests_series(
	mut commands: Commands,
	query: Populated<(Entity, &Test, &TestFunc), Without<ShouldSkip>>,
) -> Result {
	for (entity, test, func) in query.iter() {
		run_test(commands.reborrow(), entity, test, move || func.run())?;
	}
	Ok(())
}


pub(super) fn run_non_send_tests_series(
	_: NonSendMarker,
	mut commands: Commands,
	mut query: Populated<
		(Entity, &Test, &mut NonSendTestFunc),
		Without<ShouldSkip>,
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
	entity: Entity,
	test: &Test,
	func: impl FnOnce() -> Result<(), String>,
) -> Result {
	let result = PanicContext::catch(func);

	let outcome = match (result, test.should_panic) {
		(PanicResult::Ok, test::ShouldPanic::No) => {
			//ok
			TestOutcome::Pass
		}
		(PanicResult::Ok, test::ShouldPanic::Yes) => {
			//ok but should have panicked
			TestOutcome::ExpectedPanic { message: None }
		}
		(PanicResult::Ok, test::ShouldPanic::YesWithMessage(message)) => {
			//ok but should have panicked
			TestOutcome::ExpectedPanic {
				message: Some(message.to_string()),
			}
		}
		(PanicResult::Err(message), _) => {
			// errored
			TestOutcome::Err { message }
		}
		(
			PanicResult::Panic { .. },
			test::ShouldPanic::Yes | test::ShouldPanic::YesWithMessage(_),
		) => {
			// panicked and should have
			TestOutcome::Pass
		}
		(PanicResult::Panic { location, payload }, test::ShouldPanic::No) => {
			// panicked but shouldnt have
			TestOutcome::Panic { location, payload }
		}
	};
	commands.entity(entity).insert(outcome);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use test::TestDescAndFn;

	fn run_test(test: TestDescAndFn) -> TestOutcome {
		let mut app = App::new().with_plugins(TestPlugin);
		app.world_mut().spawn(tests_bundle(vec![test]));
		let store = Store::new(None);

		app.add_observer(
			move |ev: On<Insert, TestOutcome>,
			      outcomes: Query<&TestOutcome>,
			      mut commands: Commands| {
				store.set(Some(outcomes.get(ev.entity).unwrap().clone()));
				commands.write_message(AppExit::Success);
			},
		);
		app.run();
		store.get().unwrap()
	}

	#[test]
	fn works_sync() {
		run_test(test_ext::new_auto(|| Ok(()))).xpect_eq(TestOutcome::Pass);
		run_test(test_ext::new_auto(|| Err("pizza".into()))).xpect_eq(
			TestOutcome::Err {
				message: "pizza".into(),
			},
		);
		run_test(test_ext::new_auto(|| panic!("expected")).with_should_panic())
			.xpect_eq(TestOutcome::Pass);
		run_test(test_ext::new_auto(|| Ok(())).with_should_panic())
			.xpect_eq(TestOutcome::ExpectedPanic { message: None });
		run_test(
			test_ext::new_auto(|| panic!("boom"))
				.with_should_panic_message("boom"),
		)
		.xpect_eq(TestOutcome::Pass);
		run_test(
			test_ext::new_auto(|| Ok(())).with_should_panic_message("boom"),
		)
		.xpect_eq(TestOutcome::ExpectedPanic {
			message: Some("boom".into()),
		});
		run_test(test_ext::new_auto(|| panic!("pizza"))).xpect_eq(
			TestOutcome::Panic {
				payload: Some("pizza".into()),
				location: Some(FileSpan::new_with_start(
					file!(),
					line!() - 5,
					40,
				)),
			},
		);
	}
}
