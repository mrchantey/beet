use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use forky_core::ResultTEExt;

#[derive_action]
pub struct Wander;

fn wander(
	mut targets: Query<(
		&Transform,
		&Velocity,
		&mut WanderParams,
		&MaxSpeed,
		&MaxForce,
		&mut Impulse,
	)>,
	query: Query<(&TargetAgent, &Wander), (With<Running>, With<Wander>)>,
) {
	for (target, _) in query.iter() {
		if let Some((
			transform,
			velocity,
			mut wander,
			max_speed,
			max_force,
			mut impulse,
		)) = targets.get_mut(**target).ok_or(|e| log::warn!("{e}"))
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
