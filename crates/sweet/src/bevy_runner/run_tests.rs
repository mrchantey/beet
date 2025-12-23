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
		payload: String,
		/// The location of the panic if available
		location: Option<LineCol>,
	},
}

pub(super) fn run_tests_series(
	mut commands: Commands,
	query: Query<(Entity, &Test, &TestFunc), Without<ShouldSkip>>,
) -> Result {
	for (entity, test, func) in query.iter() {
		run_test(commands.reborrow(), entity, test, move || func.run())?;
	}
	Ok(())
}


pub(super) fn run_non_send_tests_series(
	_: NonSendMarker,
	mut commands: Commands,
	mut query: Query<
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
	// temp: disable legacy test runner
	let prev = std::panic::take_hook();
	// gag panic hook

	thread_local! {
		static LOCATION: std::cell::Cell<Option<LineCol>> = std::cell::Cell::new(None);
	}
	std::panic::set_hook(Box::new(|info| {
		if let Some(location) = info.location() {
			LOCATION.with(|loc| {
				loc.set(Some(LineCol::from_location(location)));
			});
		}
	}));


	// #[cfg(target_arch = "wasm32")]
	// let result = js_runtime::panic_to_error(&mut func);

	// #[cfg(not(target_arch = "wasm32"))]
	let result =
		std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || func()));

	// js_runtime::
	// let result = Ok(func());
	std::panic::set_hook(prev);

	let outcome = match (result, test.should_panic) {
		(Ok(Ok(())), test::ShouldPanic::No) => {
			//ok
			TestOutcome::Pass
		}
		(Ok(Ok(())), test::ShouldPanic::Yes) => {
			//ok but should have panicked
			TestOutcome::ExpectedPanic { message: None }
		}
		(Ok(Ok(())), test::ShouldPanic::YesWithMessage(message)) => {
			//ok but should have panicked
			TestOutcome::ExpectedPanic {
				message: Some(message.to_string()),
			}
		}
		(Ok(Err(message)), _) => {
			// errored
			TestOutcome::Err { message }
		}
		(
			Err(_),
			test::ShouldPanic::Yes | test::ShouldPanic::YesWithMessage(_),
		) => {
			// panicked and should have
			TestOutcome::Pass
		}
		(Err(payload), test::ShouldPanic::No) => {
			// panicked but shouldnt have
			TestOutcome::Panic {
				payload: panic_to_str(payload),
				location: LOCATION.with(|loc| loc.take()),
			}
		}
	};
	commands.entity(entity).insert(outcome);
	Ok(())
}



// #[cfg(target_arch = "wasm32")]
// fn panic_to_str(payload: wasm_bindgen::JsValue) -> String {
// 	if payload.is_string() {
// 		payload.as_string().unwrap()
// 	} else {
// 		format!("non-string payload: {:?}", payload)
// 	}
// }

// #[cfg(not(target_arch = "wasm32"))]
fn panic_to_str(payload: Box<dyn std::any::Any>) -> String {
	if let Some(str) = payload.downcast_ref::<&str>() {
		str.to_string()
	} else if let Some(str) = payload.downcast_ref::<String>() {
		str.clone()
	} else {
		"non-string panic payload".to_string()
	}
}


#[cfg(test)]
mod tests {
	use test::TestDescAndFn;

	use super::*;
	use crate::libtest::test_desc_ext::TestDescExt;


	fn run_test(test: TestDescAndFn) -> TestOutcome {
		let mut app = App::new().with_plugins(TestPlugin);

		insert_tests(app.world_mut(), vec![test]);
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
				payload: "pizza".into(),
				location: Some(LineCol::new(line!() - 3, 40)),
			},
		);
	}
}
