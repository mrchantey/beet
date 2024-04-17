use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

#[derive_action(Default)]
#[action(graph_role=GraphRole::Node)]
/// Inserts the given component after running for a given duration. Has no effect if
/// the action completes before the duration.
pub struct InsertInDuration<T: SettableComponent> {
	pub duration: Duration,
	pub value: T,
}

impl<T: SettableComponent> Default for InsertInDuration<T> {
	fn default() -> Self {
		Self {
			duration: Duration::from_secs(1),
			value: T::default(),
		}
	}
}

impl<T: SettableComponent> InsertInDuration<T> {
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

pub fn insert_in_duration<T: SettableComponent>(
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
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(BeetSystemsPlugin::<EcsModule, _>::default());
		app.insert_time();

		let root = InsertInDuration::<RunResult>::default()
			.into_beet_builder()
			.build(app.world_mut())
			.value;

		expect(&app).to_have_component::<Running>(root)?;

		app.update_with_secs(2);

		expect(&app).component(root)?.to_be(&RunResult::Success)?;
		expect(&app).not().to_have_component::<Running>(root)?;
		Ok(())
	}
}
