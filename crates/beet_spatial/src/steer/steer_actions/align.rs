use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Align [`Velocity`] with that of entities with the given component.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(align::<M>.in_set(TickSet))]
pub struct Align<M: GenericActionComponent> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: GenericActionComponent> Default for Align<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			phantom: PhantomData,
		}
	}
}

impl<M: GenericActionComponent> Align<M> {
	pub fn new(scalar: f32) -> Self {
		Self {
			scalar,
			phantom: PhantomData,
		}
	}
}

fn align<M: GenericActionComponent>(
	boids: Query<(Entity, &Transform, &Velocity), With<M>>,
	mut agents: Query<(Entity, &Transform, &mut Impulse, &GroupParams)>,
	query: Query<(&TargetAgent, &Align<M>), With<Running>>,
) {
	for (target, align) in query.iter() {
		let Ok((entity, transform, mut impulse, params)) =
			agents.get_mut(**target)
		else {
			continue;
		};

		let new_impulse =
			align_impulse(entity, transform.translation, params, boids.iter());

		**impulse += *new_impulse * align.scalar;
	}
}
