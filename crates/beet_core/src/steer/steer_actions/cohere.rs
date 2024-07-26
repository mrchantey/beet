use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Move towards the center of mass of entities with the given component.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(cohere::<M>.in_set(TickSet))]
pub struct Cohere<M: GenericActionComponent> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: GenericActionComponent> Default for Cohere<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			phantom: PhantomData,
		}
	}
}

impl<M: GenericActionComponent> Cohere<M> {
	pub fn new(scalar: f32) -> Self {
		Self {
			scalar,
			phantom: PhantomData,
		}
	}
}

fn cohere<M: GenericActionComponent>(
	boids: Query<(Entity, &Transform), With<M>>,
	mut agents: Query<(
		Entity,
		&Transform,
		&mut Impulse,
		&MaxSpeed,
		&GroupParams,
	)>,
	query: Query<(&TargetAgent, &Cohere<M>), With<Running>>,
) {
	for (target, cohere) in query.iter() {
		let Ok((entity, transform, mut impulse, max_speed, params)) =
			agents.get_mut(**target)
		else {
			continue;
		};

		let new_impulse = cohere_impulse(
			entity,
			transform.translation,
			*max_speed,
			params,
			boids.iter(),
		);

		**impulse += *new_impulse * cohere.scalar;
	}
}
