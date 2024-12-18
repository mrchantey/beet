use crate::prelude::*;
use bevy::prelude::*;
use rand::Rng;
use std::ops::Range;
use std::time::Duration;

/// Triggers the given event after running for a given duration. Has no effect if
/// the action completes before the duration.
#[derive(Debug, Clone, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(trigger_in_duration::<T>.in_set(TickSet))]
#[require(ContinueRun)]
pub struct TriggerInDuration<T: GenericActionEvent> {
	pub duration: Duration,
	/// Optionally randomize the duration within this range
	pub range: Option<Range<Duration>>,
	pub value: T,
}

impl<T: Default + GenericActionEvent> Default for TriggerInDuration<T> {
	fn default() -> Self { Self::new(T::default(), Duration::from_secs(1)) }
}

impl<T: GenericActionEvent> TriggerInDuration<T> {
	pub fn new(value: T, duration: Duration) -> Self {
		Self {
			value,
			duration,
			range: None,
		}
	}
	pub fn with_secs(value: T, secs: u64) -> Self {
		Self {
			value,
			duration: Duration::from_secs(secs),
			range: None,
		}
	}
	pub fn with_millis(value: T, millis: u64) -> Self {
		Self {
			value,
			duration: Duration::from_millis(millis),
			range: None,
		}
	}
	pub fn with_range(value: T, range: Range<Duration>) -> Self {
		Self {
			value,
			duration: range.start,
			range: Some(range),
		}
	}
}

pub fn trigger_in_duration<T: GenericActionEvent>(
	mut commands: Commands,
	mut query: Query<
		(Entity, &RunTimer, &mut TriggerInDuration<T>),
		With<Running>,
	>,
) {
	for (entity, timer, mut action) in query.iter_mut() {
		if timer.last_started.elapsed() >= action.duration {
			commands.entity(entity).trigger(action.value.clone());
			// Randomize the next duration if a range is provided
			if let Some(range) = &action.range {
				action.duration = rand::thread_rng().gen_range(range.clone());
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevyhub::prelude::*;
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
		)>::default())
			.configure_sets(Update, PreTickSet.before(TickSet))
			.insert_time();

		app.world_mut().spawn((
			Running,
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
