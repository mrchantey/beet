use crate::prelude::*;
use bevy::prelude::*;

/// Max force used to clamp forces
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct MaxForce(pub f32);

impl Default for MaxForce {
	fn default() -> Self { Self(0.01) }
}

/// Max speed used to clamp velocity
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct MaxSpeed(pub f32);

impl Default for MaxSpeed {
	fn default() -> Self { Self(1.) }
}

/// Scale the force effect that a particular behavior will have
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct ForceScalar(pub f32);

impl Default for ForceScalar {
	fn default() -> Self { Self(1.) }
}




/// This should be used in conjunction with the [`ForceBundle`] and [`TransformBundle`]
#[derive(Default, Bundle)]
pub struct SteerBundle {
	pub max_force: MaxForce,
	pub max_speed: MaxSpeed,
	pub arrive_radius: ArriveRadius,
	pub wander_params: WanderParams,
}

impl SteerBundle {
	pub fn scaled_to(mut self, val: f32) -> Self {
		self.max_force.0 *= val;
		self.max_speed.0 *= val;
		self.arrive_radius.0 *= val;
		self.wander_params = self.wander_params.scaled_to(val);
		self
	}

	pub fn with_target(self, target: impl Into<SteerTarget>) -> impl Bundle {
		(self, target.into())
	}
}
