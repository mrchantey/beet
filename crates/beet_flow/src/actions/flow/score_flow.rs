use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use std::cmp::Ordering;


#[derive(Event)]
pub struct RequestScore;

pub type OnChildScore = OnChildValue<ScoreValue>;


/// The score flow is a utility ai selector.
/// Children should provide a score on request, see [`ScoreProvider`].
///
#[derive(Default, Deref, DerefMut, Component, Action, Reflect)]
#[reflect(Default, Component)]
#[category(ActionCategory::ChildBehaviors)]
#[observers(on_start, on_receive_score, passthrough_run_result)]
pub struct ScoreFlow(HashMap<Entity, ScoreValue>);

fn on_start(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	mut query: Query<(&mut ScoreFlow, &Children)>,
) {
	let (mut score_flow, children) = query
		.get_mut(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	score_flow.clear();

	commands.trigger_targets(
		RequestScore,
		children.iter().cloned().collect::<Vec<_>>(),
	);
}

fn on_receive_score(
	trigger: Trigger<OnChildScore>,
	mut commands: Commands,
	mut query: Query<(&mut ScoreFlow, &Children)>,
) {
	let (mut flow, children) = query
		.get_mut(trigger.entity())
		.expect(child_expect::NO_CHILDREN);

	flow.insert(trigger.event().child(), *trigger.event().value());

	if flow.len() == children.iter().len() {
		let (highest, _) = flow
			.iter()
			.max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
			.expect(child_expect::NO_CHILDREN);
		commands.entity(*highest).trigger(OnRun);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use ::sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(
			ActionPlugin::<(ScoreFlow, ScoreProvider, EndOnRun)>::default(),
		);
		let world = app.world_mut();
		world.add_observer(bubble_run_result);

		let on_result = observe_trigger_names::<OnRunResult>(world);
		let on_run = observe_triggers::<OnRun>(world);

		world
			.spawn((Name::new("root"), ScoreFlow::default()))
			.with_children(|parent| {
				parent.spawn((
					Name::new("child1"),
					ScoreProvider::NEUTRAL,
					EndOnRun::success(),
				));
				parent.spawn((
					Name::new("child2"),
					ScoreProvider::PASS,
					EndOnRun::success(),
				));
			})
			.flush_trigger(OnRun);

		expect(&on_run).to_have_been_called_times(2);
		expect(&on_result).to_have_been_called_times(2);
		expect(&on_result).to_have_returned_nth_with(0, &"child2".to_string());
		expect(&on_result).to_have_returned_nth_with(1, &"root".to_string());
	}
}
