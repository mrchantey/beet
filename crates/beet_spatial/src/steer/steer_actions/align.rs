use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Steers boids towards the average heading of their neighbors, promoting synchronized movement.
/// This is done by updating the [`Velocity`] component.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(align::<M>.in_set(TickSet))]
#[require(ContinueRun)]
pub struct Align<M: GenericActionComponent> {
	/// The scalar to apply to the impulse
	pub scalar: f32,
	/// The radius within which to align with other boids
	pub radius: f32,
	#[reflect(ignore)]
	phantom: PhantomData<M>,
}

impl<M: GenericActionComponent> Default for Align<M> {
	fn default() -> Self {
		Self {
			scalar: 1.,
			radius: 0.5,
			phantom: PhantomData,
		}
	}
}

impl<M: GenericActionComponent> Align<M> {
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

fn align<M: GenericActionComponent>(
	boids: Query<(Entity, &Transform, &Velocity), With<M>>,
	mut agents: Query<(Entity, &Transform, &mut Impulse)>,
	query: Query<(&TargetEntity, &Align<M>), With<Running>>,
) {
	for (target, align) in query.iter() {
		let Ok((entity, transform, mut impulse)) = agents.get_mut(**target)
		else {
			continue;
		};

		**impulse +=
			*align_impulse(entity, transform.translation, align, boids.iter());
	}
}
