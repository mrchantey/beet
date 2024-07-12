use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(provide_score)]
/// Provides a [`ScoreValue`] based on distance to the [`SteerTarget`]
pub struct SteerTargetScoreProvider {
	/// fail if already at location
	pub min_radius: f32,
	pub max_radius: f32,
}

impl Default for SteerTargetScoreProvider {
	fn default() -> Self {
		Self {
			min_radius: 1.,
			max_radius: 10.,
		}
	}
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
	{
		let dist = Vec3::distance(transform.translation, target);
		if dist >= action.min_radius && dist <= action.max_radius {
			1.
		} else {
			0.
		}
	} else {
		0.
	};
	commands
		.entity(parent.get())
		.trigger(OnChildScore::new(trigger.entity(), score));
}
