use crate::prelude::*;
use bevy::prelude::*;

/// Logs the [`Name`] of the entity when it runs.
#[derive(Default, Component, Action, Reflect)]
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


/// Add this to flow actions to pass the run result to the parent.
pub fn passthrough_run_result(
	trigger: Trigger<OnChildResult>,
	mut commands: Commands,
) {
	commands.trigger_targets(
		OnRunResult::new(*trigger.event().value()),
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
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<EndOnRun>::default());
		let world = app.world_mut();
		world.observe(bubble_run_result);
		let on_run = observe_triggers::<OnRun>(world);
		let on_run_result = observe_triggers::<OnRunResult>(world);

		world
			.spawn((Name::new("root"), EndOnRun::success()))
			.observe(passthrough_run_result)
			.with_children(|parent| {
				parent
					.spawn((Name::new("child1"), EndOnRun::success()))
					// child starts running which triggers parent
					.flush_trigger(OnRun);
			});

		expect(&on_run).to_have_been_called_times(1)?;
		expect(&on_run_result).to_have_been_called_times(2)?;

		Ok(())
	}
}
