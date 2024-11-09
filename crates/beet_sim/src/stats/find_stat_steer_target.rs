use crate::prelude::*;
use beet_flow::prelude::*;
use beet_spatial::prelude::*;
use bevy::prelude::*;

/// Sets the [`SteerTarget`] when an entity with the given name is nearby.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[observers(find_steer_target)]
#[require(StatId, StatValueGoal)]
pub struct FindStatSteerTarget {}

impl Default for FindStatSteerTarget {
	fn default() -> Self { Self {} }
}

impl FindStatSteerTarget {}

// TODO this shouldnt run every frame?

fn find_steer_target(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	agents: Query<&Transform>,
	targets: Query<
		(Entity, &Transform, &StatId, &StatValue),
		Or<(With<CollectableStat>, With<ZoneStat>)>,
	>,
	query: Populated<(
		&TargetEntity,
		&FindStatSteerTarget,
		&StatId,
		&StatValueGoal,
	)>,
) {
	let (agent_entity, _action, goal_id, value_goal) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	let agent_transform = agents
		.get(**agent_entity)
		.expect(expect_action::TARGET_MISSING);

	let mut best_score = f32::MAX;
	let mut closest_target = None;

	for (pickup_entity, pickup_transform, pickup_id, pickup_value) in
		targets.iter()
	{
		if pickup_id != goal_id {
			continue;
		}
		let new_dist = Vec3::distance(
			agent_transform.translation,
			pickup_transform.translation,
		);

		let new_value =
			match (value_goal, f32::is_sign_positive(pickup_value.0)) {
				(StatValueGoal::High, true) => pickup_value.0,
				(StatValueGoal::Low, false) => pickup_value.0,
				// this pickup would work against the goal, so ignore it
				_ => continue,
			};

		let new_score = new_dist + new_value;

		if new_score < best_score {
			best_score = new_score;
			closest_target = Some(pickup_entity);
		}
	}

	if let Some(closest_target) = closest_target {
		commands
			.entity(**agent_entity)
			.insert(SteerTarget::Entity(closest_target));
	}
}
