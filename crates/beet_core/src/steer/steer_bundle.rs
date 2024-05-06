use crate::prelude::*;
use bevy::prelude::*;

#[derive(Default, Clone, Component, Reflect)]
/// Default marker for agents that should be considered
/// in group steering actions.
pub struct GroupSteerAgent;

/// Max force used to clamp [`Force`] and [`Impulse`]
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct MaxForce(pub f32);

impl Default for MaxForce {
	fn default() -> Self { Self(0.01) }
}

/// Max speed used as a scalar for steering and to clamp [`Velocity`]
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
	pub group_params: GroupParams,
}

impl SteerBundle {
	/// Defaults are in a range 0..1, this is a convenience method for scaling all parameters in the bundle.
	/// For instance if using pixel space, you might want to scale all parameters by 100.
	pub fn scaled_to(mut self, val: f32) -> Self {
		self.max_force.0 *= val;
		self.max_speed.0 *= val;
		self.arrive_radius.0 *= val;
		self.wander_params = self.wander_params.scaled_to(val);
		self.group_params = self.group_params.scaled_to(val);
		self
	}

	pub fn with_target(self, target: impl Into<SteerTarget>) -> impl Bundle {
		(self, target.into())
	}
}
