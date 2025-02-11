use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Triggers the given event after running for a given duration. Has no effect if
/// the action completes before the duration.
#[derive(Debug, Clone, Component, Reflect)]
#[require(ContinueRun)]
pub struct ReturnInDuration<T: ResultPayload = RunResult> {
	pub duration: Duration,
	pub value: T,
}

impl<T: Default + ResultPayload> Default for ReturnInDuration<T> {
	fn default() -> Self { Self::new(T::default(), Duration::from_secs(1)) }
}

impl<T: ResultPayload> ReturnInDuration<T> {
	pub fn new(value: T, duration: Duration) -> Self {
		Self { value, duration }
	}
	pub fn with_secs(value: T, secs: u64) -> Self {
		Self {
			value,
			duration: Duration::from_secs(secs),
		}
	}
	pub fn with_millis(value: T, millis: u64) -> Self {
		Self {
			value,
			duration: Duration::from_millis(millis),
		}
	}
}

pub fn return_in_duration<T: ResultPayload>(
	mut commands: Commands,
	mut query: Populated<
		(Entity, &RunTimer, &mut ReturnInDuration<T>),
		With<Running>,
	>,
) {
	for (entity, timer, action) in query.iter_mut() {
		if timer.last_started.elapsed() >= action.duration {
			commands
				.trigger(OnResultAction::global(entity, action.value.clone()));
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use std::time::Duration;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();

		let on_result = observe_triggers::<OnResult>(app.world_mut());

		app.add_plugins(BeetFlowPlugin::default()).insert_time();

		app.world_mut().spawn((
			Running,
			ReturnInDuration::new(RunResult::Success, Duration::from_secs(2)),
		));

		app.update_with_secs(1);

		expect(&on_result).not().to_have_been_called();

		app.update_with_secs(10);

		expect(&on_result).to_have_been_called();
	}
}
