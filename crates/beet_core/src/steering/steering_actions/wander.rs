use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;

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
		let (
			transform,
			velocity,
			mut wander,
			max_speed,
			max_force,
			mut impulse,
		) = targets.get_mut(**target).unwrap();


		*impulse = wander_impulse(
			&transform.translation,
			&velocity,
			&mut wander,
			*max_speed,
			*max_force,
		);
	}
}
