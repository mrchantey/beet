use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;


#[derive(Default, Clone, Component, Reflect)]
/// Default marker for group steering actions
pub struct GroupSteerAgent;



#[derive_action(Default, Clone)]
#[action(graph_role=GraphRole::Agent)]
/// Align with entities that have a [`Transform`], [`Velocity`] and [``]
pub struct Align<M: SettableComponent + FromReflect> {
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: SettableComponent + FromReflect> Default for Align<M> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}
impl<M: SettableComponent + FromReflect> Clone for Align<M> {
	fn clone(&self) -> Self {
		Self {
			phantom: self.phantom.clone(),
		}
	}
}


fn align<M: SettableComponent + FromReflect>(
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
