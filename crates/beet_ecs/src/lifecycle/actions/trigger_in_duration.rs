use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Triggers the given event after running for a given duration. Has no effect if
/// the action completes before the duration.
/// # Requires
/// - [`RunTimer`]
#[derive(Debug, Clone, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(trigger_in_duration::<T>.in_set(TickSet))]
pub struct TriggerInDuration<T: GenericActionEvent> {
	pub duration: Duration,
	pub value: T,
}

impl<T: Default + GenericActionEvent> Default for TriggerInDuration<T> {
	fn default() -> Self { Self::new(T::default(), Duration::from_secs(1)) }
}

impl<T: GenericActionEvent> TriggerInDuration<T> {
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

pub fn trigger_in_duration<T: GenericActionEvent>(
	mut commands: Commands,
	mut query: Query<(Entity, &RunTimer, &TriggerInDuration<T>), With<Running>>,
) {
	for (entity, timer, insert_in_duration) in query.iter_mut() {
		println!("here!{:?}", timer.last_started.elapsed());
		if timer.last_started.elapsed() >= insert_in_duration.duration {
			commands
				.entity(entity)
				.trigger(insert_in_duration.value.clone());
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use std::time::Duration;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();

		let on_result = observe_triggers::<OnRunResult>(app.world_mut());

		app.add_plugins(ActionPlugin::<(
			RunTimer,
			TriggerInDuration<OnRunResult>,
		)>::default());

		app.configure_sets(Update, PreTickSet.before(TickSet));

		app.insert_time();

		app.world_mut().spawn((
			Running,
			RunTimer::default(),
			TriggerInDuration::<OnRunResult>::new(
				OnRunResult::default(),
				Duration::from_secs(2),
			),
		));

		app.update_with_secs(1);

		expect(&on_result).not().to_have_been_called()?;

		app.update_with_secs(10);

		expect(&on_result).to_have_been_called()?;

		Ok(())
	}
}
