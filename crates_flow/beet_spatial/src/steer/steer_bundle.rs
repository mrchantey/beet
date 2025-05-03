use crate::prelude::*;
use bevy::prelude::*;

#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
/// Default marker for agents that should be considered
/// in group steering actions.
pub struct GroupSteerAgent;

/// Max force used to clamp [`Force`] and [`Impulse`].
/// Higher values will make the agent more responsive to steering forces,
/// appropriate for walking agents.
/// Lower values will give a more spongey feel, appropriate for boats, cars etc.
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
#[require(Velocity, Force, Impulse)]
pub struct MaxForce(pub f32);

impl Default for MaxForce {
	fn default() -> Self { Self(0.01) }
}

/// Max speed used as a scalar for steering and to clamp [`Velocity`]
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
#[require(Velocity, Force, Impulse)]
pub struct MaxSpeed(pub f32);

impl Default for MaxSpeed {
	fn default() -> Self { Self(1.) }
}

/// Scale the force effect that a particular behavior will have
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
#[require(Velocity, Force, Impulse)]
pub struct ForceScalar(pub f32);

impl Default for ForceScalar {
	fn default() -> Self { Self(1.) }
}



/// The components required for steering behaviors.
/// This should be used in conjunction with the [`ForceBundle`] and [`TransformBundle`]
#[derive(Default, Bundle)]
pub struct SteerBundle {
	/// The maximum force that can be applied to the agent,
	/// see [`integrate_force`] for example usage
	pub max_force: MaxForce,
	/// The maximum speed that the agent can move,
	/// see [`arrive_speed`] for example usage
	pub max_speed: MaxSpeed,
	/// The radius at which the agent will begin to slow down,
	/// see [`arrive_speed`] for example usage
	pub arrive_radius: ArriveRadius,
}

impl SteerBundle {
	/// Defaults are in a range 0..1, this is a convenience method for scaling all parameters in the bundle.
	/// For instance if using pixel space, you might want to scale all parameters by 100.
	pub fn scaled_dist(mut self, val: f32) -> Self {
		self.max_force.0 *= val;
		self.max_speed.0 *= val;
		self.arrive_radius.0 *= val;
		self
	}
}
