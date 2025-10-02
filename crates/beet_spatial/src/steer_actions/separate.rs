use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use std::marker::PhantomData;

/// Ensures that boids avoid crowding neighbors by maintaining a minimum distance from each other.
/// This is done by updating the [`Velocity`] component.
/// ## Tags
/// - [LongRunning](ActionTag::LongRunning)
/// - [MutateOrigin](ActionTag::MutateOrigin)
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component)]
#[require(ContinueRun)]
pub struct Separate<M> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	/// The radius within which to avoid other boids
	pub radius: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M> Default for Separate<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			radius: 0.25,
			phantom: PhantomData,
		}
	}
}

impl<M> Separate<M> {
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

pub(crate) fn separate<M: Component>(
	boids: Query<(Entity, &Transform), With<M>>,
	mut agents: AgentQuery<(Entity, &Transform, &mut Impulse, &MaxSpeed)>,
	query: Query<(Entity, &Running, &Separate<M>)>,
) -> Result {
	for (action, running, separate) in query.iter() {
		let (entity, transform, mut impulse, max_speed) =
			agents.get_mut(action)?;

		**impulse += *separate_impulse(
			entity,
			transform.translation,
			*max_speed,
			separate,
			boids.iter(),
		);
	}
	Ok(())
}
