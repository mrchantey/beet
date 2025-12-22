use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::NonSendMarker;

/// the error message
#[derive(Debug, Component)]
#[component(storage = "SparseSet")]
pub enum TestOutcome {
	Pass,
	/// The test returned an [`Err(String)`]
	Err(String),
	/// The test panicked
	Panic {
		info: String,
	},
	/// The test did not panic but was expected to
	ExpectedPanic,
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
		run_test(commands.reborrow(), entity, test, move || func.run())?;
	}
	Ok(())
}


fn run_test(
	mut commands: Commands,
	entity: Entity,
	_test: &Test,
	func: impl FnOnce() -> Result<(), String>,
) -> Result {
	// TODO async
	// TODO panic
	let result = match func() {
		Ok(_) => TestOutcome::Pass,
		Err(err) => TestOutcome::Err(err),
	};
	commands.entity(entity).insert(result);
	Ok(())
}


#[cfg(test)]
mod tests {
	use super::*;


	#[test]
	fn works() {
		let mut app = App::new().with_plugins(TestPlugin);

		let _root = insert_tests(app.world_mut(), vec![
			test_ext::anon(|| {
				// panic!("it panicked");
				Ok(())
			}),
			test_ext::anon(|| Ok(())),
		]);
		app.add_systems(Update, |mut commands: Commands| {
			commands.write_message(AppExit::Success);
		});
		app.run().xpect_eq(AppExit::Success);
	}
}
