use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_math::Vec3;




// TODO this should be generic
#[derive(Default, Deref, DerefMut)]
#[derive_action]
pub struct SetVelocity(pub Vec3);

impl SetVelocity {
	pub fn new(value: Vec3) -> Self { Self(value) }
}

fn set_velocity(
	mut agents: Query<&mut Velocity>,
	query: Query<(&TargetAgent, &SetVelocity), Added<Running>>,
) {
	for (entity, value) in query.iter() {
		if let Ok(mut velocity) = agents.get_mut(**entity) {
			*velocity = Velocity(value.0.clone());
		}
	}
}
