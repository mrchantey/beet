use crate::prelude::*;
use bevy::prelude::*;
use std::cmp::Ordering;

#[derive(Default, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(on_start, passthrough_run_result)]
pub struct ScoreFlow;

fn on_start(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<&Children>,
	scores: Query<&Score>,
) {
	let children = query
		.get(trigger.entity())
		.expect(child_expect::NO_CHILDREN);

	if let Some(highest) = get_highest(scores, children) {
		commands.trigger_targets(OnRun, highest);
	} else {
		commands.trigger_targets(OnRunResult::success(), trigger.entity());
	}
}

fn get_highest(scores: Query<&Score>, children: &Children) -> Option<Entity> {
	children
		.iter()
		.filter_map(|&child| scores.get(child).ok().map(|score| (child, score)))
		.max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
		.map(|(entity, _)| entity)
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
		app.add_plugins(ActionPlugin::<(ScoreFlow, EndOnRun)>::default());
		let world = app.world_mut();
		world.observe(bubble_run_result);

		let on_result = observe_trigger_names::<OnRunResult>(world);
		let on_run = observe_triggers::<OnRun>(world);

		world
			.spawn((Name::new("root"), ScoreFlow))
			.with_children(|parent| {
				parent.spawn((
					Name::new("child1"),
					Score::neutral(),
					EndOnRun::success(),
				));
				parent.spawn((
					Name::new("child2"),
					Score::Pass,
					EndOnRun::success(),
				));
			})
			.flush_trigger(OnRun);

		expect(&on_run).to_have_been_called_times(2)?;
		expect(&on_result).to_have_been_called_times(2)?;
		expect(&on_result)
			.to_have_returned_nth_with(0, &"child2".to_string())?;
		expect(&on_result).to_have_returned_nth_with(1, &"root".to_string())?;

		Ok(())
	}
}
