use crate::prelude::*;
use bevy::prelude::*;

/// Logs the [`Name`] of the entity when it runs.
#[derive(Default, Action, Reflect)]
#[reflect(Default, Component)]
#[observers(log_name_on_run)]
pub struct BubbleRunResult;

pub fn bubble_run_result(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	parents: Query<&Parent>,
) {
	if let Some(parent) = parents.get(trigger.entity()).ok() {
		commands.trigger_targets(
			OnChildResult::new(trigger.entity(), trigger.event().result()),
			parent.get(),
		);
	}
}

pub fn passthrough_run_result(
	trigger: Trigger<OnChildResult>,
	mut commands: Commands,
) {
	commands.trigger_targets(
		OnRunResult::new(trigger.event().result()),
		trigger.entity(),
	);
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;



	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();
		world.observe(bubble_run_result);
		world.observe(trigger_run_on_spawn);
		let on_run = observe_triggers::<OnRun>(&mut world);
		let on_run_result = observe_triggers::<OnRunResult>(&mut world);

		world
			.spawn((Name::new("root"), EndOnRun::success()))
			.observe(passthrough_run_result)
			.with_children(|parent| {
				// child starts running which triggers parent
				parent.spawn((
					Name::new("child1"),
					RunOnSpawn,
					EndOnRun::success(),
				));
			});
		world.flush();

		expect(&on_run).to_have_been_called_times(1)?;
		expect(&on_run_result).to_have_been_called_times(2)?;

		Ok(())
	}
}
