use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Ensures that boids avoid crowding neighbors by maintaining a minimum distance from each other.
/// This is done by updating the [`Velocity`] component.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(separate::<M>.in_set(TickSet))]
pub struct Separate<M: GenericActionComponent> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	/// The radius within which to avoid other boids
	pub radius: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: GenericActionComponent> Default for Separate<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			radius: 0.25,
			phantom: PhantomData,
		}
	}
}

impl<M: GenericActionComponent> Separate<M> {
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

fn separate<M: GenericActionComponent>(
	boids: Query<(Entity, &Transform), With<M>>,
	mut agents: Query<(Entity, &Transform, &mut Impulse, &MaxSpeed)>,
	query: Query<(&TargetAgent, &Separate<M>), With<Running>>,
) {
	for (target, separate) in query.iter() {
		let Ok((entity, transform, mut impulse, max_speed)) =
			agents.get_mut(**target)
		else {
			continue;
		};

		**impulse += *separate_impulse(
			entity,
			transform.translation,
			*max_speed,
			separate,
			boids.iter(),
		);
	}
}
