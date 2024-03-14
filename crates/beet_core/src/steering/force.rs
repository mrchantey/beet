use beet_ecs::exports::Reflect;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_time::Time;
use bevy_transform::prelude::*;


/// A vector measured in (m/s)
#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component)]
pub struct Velocity(pub Vec3);
/// A force that is cleared each frame.
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut, Component)]
pub struct Impulse(pub Vec3);
/// A constant force that is cleared each frame.
#[derive(Debug, Default, Clone, PartialEq, Deref, DerefMut, Component)]
pub struct Force(pub Vec3);
/// A constant force that is cleared each frame.
#[derive(Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component)]
pub struct Mass(pub f32);

impl Default for Mass {
	fn default() -> Self { Self(1.0) }
}


#[derive(Default, Bundle)]
pub struct ForceBundle {
	pub mass: Mass,
	pub velocity: Velocity,
	pub impulse: Impulse,
	pub force: Force,
}



/// Implementation of position, velocity, force integration
/// as described by Daniel Shiffman
/// https://natureofcode.com/vectors/#acceleration
pub fn integrate_force(
	time: Res<Time>,
	mut query: Query<(
		&mut Transform,
		Option<&Mass>,
		&mut Velocity,
		Option<&Force>,
		Option<&mut Impulse>,
	)>,
) {
	for (mut transform, mass, mut velocity, force, mut impulse) in
		query.iter_mut()
	{
		let mut force = force.map(|f| **f).unwrap_or_default();
		let mass = mass.map(|m| **m).unwrap_or(1.0);
		if let Some(impulse) = impulse.as_mut() {
			force += ***impulse;
			***impulse = Vec3::ZERO;
		}
		let acceleration = force / mass;
		**velocity += acceleration;
		transform.translation += **velocity * time.delta_seconds();
	}
}
