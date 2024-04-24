use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive_action(Default)]
#[action(graph_role=GraphRole::Agent)]
/// Align with entities that have a [`Transform`], [`Velocity`] and [``]
pub struct Align<M: Component + FromReflect + GetTypeRegistration> {
	phantom: PhantomData<M>,
}
impl<M: Component + FromReflect + GetTypeRegistration> Default for Align<M> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}


fn align<M: Component + FromReflect + GetTypeRegistration>(
	boids: Query<(&Transform, &Velocity), With<M>>,
	mut agents: Query<(
		&Transform,
		&mut Impulse,
		&MaxSpeed,
		&MaxForce,
		&GroupParams,
	)>,
	query: Query<(&TargetAgent, &Align<M>), With<Running>>,
) {
	for (target, _) in query.iter() {
		let Ok((transform, mut impulse, max_speed, max_force, params)) =
			agents.get_mut(**target)
		else {
			continue;
		};
		impulse.set_if_neq(align_impulse(
			&transform.translation,
			max_speed,
			max_force,
			params,
			boids.iter(),
		));
	}
}
