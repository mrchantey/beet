use crate::prelude::*;
use bevy::prelude::*;
use std::time::Duration;

#[derive_action(Default)]
#[action(graph_role=GraphRole::Node)]
pub struct SucceedInDuration {
	pub duration: Duration,
}

impl Default for SucceedInDuration {
	fn default() -> Self {
		Self {
			duration: Duration::from_secs(1),
		}
	}
}

impl SucceedInDuration {
	pub fn new(duration: Duration) -> Self { Self { duration } }
	pub fn with_secs(secs: u64) -> Self {
		Self {
			duration: Duration::from_secs(secs),
		}
	}
	pub fn with_millis(millis: u64) -> Self {
		Self {
			duration: Duration::from_millis(millis),
		}
	}
}

pub fn succeed_in_duration(
	mut commands: Commands,
	mut query: Query<(Entity, &RunTimer, &SucceedInDuration), With<Running>>,
) {
	for (entity, timer, succeed_in_duration) in query.iter_mut() {
		if timer.last_started.elapsed() >= succeed_in_duration.duration {
			commands.entity(entity).insert(RunResult::Success);
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

		let root = SucceedInDuration::default()
			.into_beet_builder()
			.spawn_no_target(app.world_mut())
			.value;

		expect(&app).to_have_component::<Running>(root)?;

		app.update_with_secs(2);

		expect(&app).component(root)?.to_be(&RunResult::Success)?;
		expect(&app).not().to_have_component::<Running>(root)?;
		Ok(())
	}
}
