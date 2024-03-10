use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy_transform::components::Transform;

#[action(system=seek)]
#[derive(Default)]
pub struct Seek;


fn seek(
	transforms: Query<&Transform>,
	mut targets: Query<(
		&Transform,
		&Velocity,
		&SteerTarget,
		&MaxSpeed,
		&MaxForce,
		&mut Impulse,
		Option<&ArriveRadius>,
	)>,
	query: Query<(&TargetAgent, &Seek), With<Running>>,
) {
	for (target, _) in query.iter() {
		let (
			transform,
			velocity,
			steer_target,
			max_speed,
			max_force,
			mut impulse,
			arrive_radius,
		) = targets.get_mut(**target).unwrap();

		let target_position = steer_target.position(&transforms).unwrap();

		*impulse = seek_impulse(
			&transform.translation,
			&velocity,
			&target_position,
			*max_speed,
			*max_force,
			arrive_radius.copied(),
		);
	}
}
