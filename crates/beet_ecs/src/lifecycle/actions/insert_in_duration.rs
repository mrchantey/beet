use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

/// Inserts the given component after running for a given duration. Has no effect if
/// the action completes before the duration.
/// # Requires
/// - [`RunTimer`]
#[derive(Debug, Clone, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(insert_in_duration::<T>.in_set(TickSet))]
pub struct InsertInDuration<T: GenericActionComponent> {
	pub duration: Duration,
	pub value: T,
}

impl<T: Default + GenericActionComponent> Default for InsertInDuration<T> {
	fn default() -> Self { Self::new(T::default(), Duration::from_secs(1)) }
}

impl<T: GenericActionComponent> InsertInDuration<T> {
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

pub fn insert_in_duration<T: GenericActionComponent>(
	mut commands: Commands,
	mut query: Query<(Entity, &RunTimer, &InsertInDuration<T>), With<Running>>,
) {
	for (entity, timer, insert_in_duration) in query.iter_mut() {
		if timer.last_started.elapsed() >= insert_in_duration.duration {
			commands
				.entity(entity)
				.insert(insert_in_duration.value.clone());
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
		app.add_plugins((
			LifecycleSystemsPlugin,
			ActionPlugin::<InsertInDuration<RunResult>>::default(),
		));
		app.insert_time();

		let root = app
			.world_mut()
			.spawn((
				Running,
				RunTimer::default(),
				InsertInDuration::<RunResult>::new(
					RunResult::default(),
					Duration::from_secs(1),
				),
			))
			.id();

		expect(&app).to_have_component::<Running>(root)?;

		app.update_with_secs(2);

		expect(&app).component(root)?.to_be(&RunResult::Success)?;
		expect(&app).not().to_have_component::<Running>(root)?;
		Ok(())
	}
}
