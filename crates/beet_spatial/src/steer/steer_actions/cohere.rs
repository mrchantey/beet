use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Encourages boids to move towards the average position of their neighbors, keeping the flock together.
/// This is done by updating the [`Velocity`] component.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(cohere::<M>.in_set(TickSet))]
#[require(ContinueRun)]
pub struct Cohere<M: GenericActionComponent> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	/// The radius within which to cohere with other boids
	pub radius: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: GenericActionComponent> Default for Cohere<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			radius: 0.7,
			phantom: PhantomData,
		}
	}
}

impl<M: GenericActionComponent> Cohere<M> {
	/// Set impulse strength with default radius
	pub fn new(scalar: f32) -> Self {
		Self {
			scalar,
			..default()
		}
	}
	/// Scale all radius and distances by this value
	pub fn scaled_dist(mut self, dist: f32) -> Self {
		self.radius *= dist;
		self
	}
}

fn cohere<M: GenericActionComponent>(
	boids: Query<(Entity, &Transform), With<M>>,
	mut agents: Query<(Entity, &Transform, &mut Impulse, &MaxSpeed)>,
	query: Query<(&TargetEntity, &Cohere<M>), With<Running>>,
) {
	for (target, cohere) in query.iter() {
		let Ok((entity, transform, mut impulse, max_speed)) =
			agents.get_mut(**target)
		else {
			continue;
		};

		**impulse += *cohere_impulse(
			entity,
			transform.translation,
			*max_speed,
			cohere,
			boids.iter(),
		);
	}
}
