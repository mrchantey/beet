use crate::prelude::*;
use bevy::prelude::*;

#[derive(Default, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(sequence_start, sequence_next)]
pub struct SequenceFlow;

fn sequence_start(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<&Children>,
) {
	if let Ok(children) = query.get(trigger.entity()) {
		if let Some(first_child) = children.iter().next() {
			commands.trigger_targets(OnRun, *first_child);
		}
	}
}
fn sequence_next(
	trigger: Trigger<OnChildResult>,
	mut commands: Commands,
	query: Query<&Children>,
) {
	if trigger.event().result() == RunResult::Failure {
		commands.trigger_targets(OnRunResult::failure(), trigger.entity());
		return;
	}
	if let Ok(children) = query.get(trigger.entity()) {
		let index = children
			.iter()
			.position(|&x| x == trigger.event().child())
			.expect("Only children may trigger OnChildResult");
		if index == children.len() - 1 {
			commands.trigger_targets(OnRunResult::success(), trigger.entity());
		} else {
			commands.trigger_targets(OnRun, children[index + 1]);
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
		let mut world = World::new();
		world.observe(bubble_run_result);

		let on_result = observe_trigger_names::<OnRunResult>(&mut world);
		let on_run = observe_triggers::<OnRun>(&mut world);

		world
			.spawn((Name::new("root"), SequenceFlow))
			.with_children(|parent| {
				parent.spawn((Name::new("child1"), EndOnRun::success()));
				parent.spawn((Name::new("child2"), EndOnRun::success()));
			})
			.flush_trigger(OnRun);

		expect(&on_run).to_have_been_called_times(3)?;
		expect(&on_result).to_have_been_called_times(3)?;
		expect(&on_result)
			.to_have_returned_nth_with(0, &"child1".to_string())?;
		expect(&on_result)
			.to_have_returned_nth_with(1, &"child2".to_string())?;
		expect(&on_result).to_have_returned_nth_with(2, &"root".to_string())?;

		Ok(())
	}
}
