use beet_core::prelude::*;

#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
/// A vector measured in (m/s).
/// This is multiplied by delta time.
/// This is the only component alongside [`Transform`] required for force integration.
pub struct Velocity(pub Vec3);


impl Velocity {
	/// Create a new velocity from the given x, y, and z values.
	pub fn from_xyz(x: f32, y: f32, z: f32) -> Self { Self(Vec3::new(x, y, z)) }
}

#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Velocity)]
/// A constant value for constraining axes
pub struct VelocityScalar(pub Vec3);

impl Default for VelocityScalar {
	fn default() -> Self { Self(Vec3::ONE) }
}

#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
#[require(Velocity)]
/// An instant force, ie jump, that is cleared each frame.
/// This is not multiplied by delta time.
pub struct Impulse(pub Vec3);
#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
#[require(Velocity)]
/// A constant force, ie gravity, that is cleared each frame.
/// This is multiplied by delta time.
pub struct Force(pub Vec3);

#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
#[require(Velocity)]
/// Larger masses are less effected by forces, defaults to `1.0`.
pub struct Mass(pub f32);

impl Default for Mass {
	fn default() -> Self { Self(1.0) }
}

/// The components required for force integration, for use with a [`TransformBundle`] or equivalent.
#[allow(missing_docs)]
#[derive(Default, Bundle)]
pub struct ForceBundle {
	pub mass: Mass,
	pub velocity: Velocity,
	pub impulse: Impulse,
	pub force: Force,
}
