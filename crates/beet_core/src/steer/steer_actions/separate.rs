use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Separate from entities with the given component.
#[derive(Debug, Clone, PartialEq, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(separate::<M>.in_set(TickSet))]
pub struct Separate<M: GenericActionComponent> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: GenericActionComponent> Default for Separate<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			phantom: PhantomData,
		}
	}
}

impl<M: GenericActionComponent> Separate<M> {
	pub fn new(scalar: f32) -> Self {
		Self {
			scalar,
			phantom: PhantomData,
		}
	}
}

fn separate<M: GenericActionComponent>(
	boids: Query<(Entity, &Transform), With<M>>,
	mut agents: Query<(
		Entity,
		&Transform,
		&mut Impulse,
		&MaxSpeed,
		&GroupParams,
	)>,
	query: Query<(&TargetAgent, &Separate<M>), With<Running>>,
) {
	for (target, separate) in query.iter() {
		let Ok((entity, transform, mut impulse, max_speed, params)) =
			agents.get_mut(**target)
		else {
			continue;
		};

		let new_impulse = separate_impulse(
			entity,
			transform.translation,
			*max_speed,
			params,
			boids.iter(),
		);

		**impulse += *new_impulse * separate.scalar;
	}
}
