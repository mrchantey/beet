use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(provide_score)]
/// Provides a [`ScoreValue`] based on distance to the [`SteerTarget`]
pub struct SteerTargetScoreProvider {
	pub radius: f32,
}

impl Default for SteerTargetScoreProvider {
	fn default() -> Self { Self { radius: 0.5 } }
}

impl SteerTargetScoreProvider {
	pub fn new(radius: f32) -> Self { Self { radius } }
}

fn provide_score(
	trigger: Trigger<RequestScore>,
	mut commands: Commands,
	transforms: Query<&Transform>,
	agents: Query<(&Transform, &SteerTarget)>,
	query: Query<(&SteerTargetScoreProvider, &TargetAgent, &Parent)>,
) {
	let (action, agent, parent) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let score = if let Ok((transform, target)) = agents.get(**agent)
		&& let Ok(target) = target.position(&transforms)
		&& Vec3::distance(transform.translation, target) <= action.radius
	{
		1.
	} else {
		0.
	};
	commands.trigger_targets(
		OnChildScore::new(trigger.entity(), score),
		parent.get(),
	);
}
