use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Encourages boids to move towards the average position of their neighbors, keeping the flock together.
/// This is done by updating the [`Velocity`] component.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Cohere<M> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	/// The radius within which to cohere with other boids
	pub radius: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M> Default for Cohere<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			radius: 0.7,
			phantom: PhantomData,
		}
	}
}

impl<M> Cohere<M> {
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

pub(crate) fn cohere<M: Component>(
	boids: Query<(Entity, &Transform), With<M>>,
	mut agents: Query<(Entity, &Transform, &mut Impulse, &MaxSpeed)>,
	query: Query<(&Running, &Cohere<M>)>,
) {
	for (running, cohere) in query.iter() {
		let (entity, transform, mut impulse, max_speed) = agents
			.get_mut(running.origin)
			.expect(&expect_action::to_have_origin(&running));


		**impulse += *cohere_impulse(
			entity,
			transform.translation,
			*max_speed,
			cohere,
			boids.iter(),
		);
	}
}
