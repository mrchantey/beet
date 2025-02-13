use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Triggers the given event after running for a given duration.
/// This has no effect if the action completes before the duration.
///
/// The default duration is 1 second.
/// ## Example
/// ```
/// # use beet_flow::prelude::*;
/// # let mut world = world();
/// # use std::time::Duration;
/// world.spawn((
///		Running,
///		ReturnInDuration::new(RunResult::Success, Duration::from_secs(2)),
///	));
///
/// ```
#[derive(Debug, Clone, Component, Reflect)]
#[require(ContinueRun)]
pub struct ReturnInDuration<T: ResultPayload = RunResult> {
	/// The length of time the action will run for before triggering the event.
	pub duration: Duration,
	/// The payload to return with
	pub payload: T,
}

impl<T: Default + ResultPayload> Default for ReturnInDuration<T> {
	fn default() -> Self { Self::new(T::default(), Duration::from_secs(1)) }
}

impl<T: ResultPayload> ReturnInDuration<T> {
	/// Specify the payload and duration
	pub fn new(payload: T, duration: Duration) -> Self {
		Self { payload, duration }
	}
	/// Specify the payload and duration in seconds
	pub fn with_secs(payload: T, secs: u64) -> Self {
		Self {
			payload,
			duration: Duration::from_secs(secs),
		}
	}
	/// Specify the payload and duration in milliseconds
	pub fn with_millis(payload: T, millis: u64) -> Self {
		Self {
			payload,
			duration: Duration::from_millis(millis),
		}
	}
}

pub(crate) fn return_in_duration<T: ResultPayload>(
	mut commands: Commands,
	mut query: Populated<
		(Entity, &RunTimer, &mut ReturnInDuration<T>),
		With<Running>,
	>,
) {
	for (entity, timer, action) in query.iter_mut() {
		if timer.last_started.elapsed() >= action.duration {
			commands.trigger(OnResultAction::global(
				entity,
				action.payload.clone(),
			));
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
		app.add_plugins(BeetFlowPlugin::default()).insert_time();

		let on_result = observe_triggers::<OnResult>(app.world_mut());


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
