use crate::prelude::*;
use bevy::prelude::*;


/// An action that runs all of its children in parallel.
/// - All results will bubble up, so expect multiple [`OnRunResult`] triggers
#[derive(Default, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(on_start, passthrough_run_result)]
pub struct ParallelFlow;

fn on_start(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<&Children>,
) {
	let children = query
		.get(trigger.entity())
		.expect(child_expect::NO_CHILDREN);
	commands
		.trigger_targets(OnRun, children.iter().cloned().collect::<Vec<_>>());
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
		app.add_plugins(ActionPlugin::<(ParallelFlow, EndOnRun)>::default());
		let world = app.world_mut();
		world.observe(bubble_run_result);

		let on_result = observe_trigger_names::<OnRunResult>(world);
		let on_run = observe_triggers::<OnRun>(world);

		world
			.spawn((Name::new("root"), ParallelFlow))
			.with_children(|parent| {
				parent.spawn((Name::new("child1"), EndOnRun::success()));
				parent.spawn((Name::new("child2"), EndOnRun::success()));
			})
			.flush_trigger(OnRun);

		expect(&on_run).to_have_been_called_times(3)?;
		expect(&on_result).to_have_been_called_times(4)?;
		expect(&on_result)
			.to_have_returned_nth_with(0, &"child1".to_string())?;
		expect(&on_result).to_have_returned_nth_with(1, &"root".to_string())?;
		expect(&on_result)
			.to_have_returned_nth_with(2, &"child2".to_string())?;
		expect(&on_result).to_have_returned_nth_with(3, &"root".to_string())?;

		Ok(())
	}
}
