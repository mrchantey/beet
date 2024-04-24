use crate::prelude::*;
use bevy::prelude::*;

/// Max force used to clamp forces, defaults to `0.1`
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct MaxForce(pub f32);

impl Default for MaxForce {
	fn default() -> Self { Self(0.1) }
}

/// Max speed used to clamp velocity, defaults to `1.0`
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct MaxSpeed(pub f32);

impl Default for MaxSpeed {
	fn default() -> Self { Self(1.0) }
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
	pub fn with_target(self, target: impl Into<SteerTarget>) -> impl Bundle {
		// self.steer_target = target.into();
		(self, target.into())
	}
}
