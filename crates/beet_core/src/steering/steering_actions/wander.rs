use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;

#[derive_action]
#[action(graph_role=GraphRole::Agent)]
pub struct Wander;

fn wander(
	mut agents: Query<(
		&Transform,
		&Velocity,
		&mut WanderParams,
		&MaxSpeed,
		&MaxForce,
		&mut Impulse,
	)>,
	query: Query<(&TargetAgent, &Wander), (With<Running>, With<Wander>)>,
) {
	for (agent, _) in query.iter() {
		if let Some((
			transform,
			velocity,
			mut wander,
			max_speed,
			max_force,
			mut impulse,
		)) = agents.get_mut(**agent).ok_or(|e| log::warn!("{e}"))
		{
			*impulse = wander_impulse(
				&transform.translation,
				&velocity,
				&mut wander,
				*max_speed,
				*max_force,
			);
		}
	}
}
