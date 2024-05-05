use bevy::prelude::*;

#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
/// A vector measured in (m/s).
/// This is multiplied by delta time.
pub struct Velocity(pub Vec3);

#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, Default)]
/// A constant value for constraining axes
pub struct VelocityScalar(pub Vec3);

impl Default for VelocityScalar {
	fn default() -> Self { Self(Vec3::ONE) }
}

#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
/// An instant force, ie jump, that is cleared each frame.
/// This is not multiplied by delta time.
pub struct Impulse(pub Vec3);
#[derive(
	Debug, Default, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
/// A constant force, ie gravity, that is cleared each frame.
/// This is multiplied by delta time.
pub struct Force(pub Vec3);

#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
/// Larger masses are less effected by forces, defaults to `1.0`.
pub struct Mass(pub f32);

impl Default for Mass {
	fn default() -> Self { Self(1.0) }
}

/// The components required for force integration, for use with a [`TransformBundle`] or equivalent.
#[derive(Default, Bundle)]
pub struct ForceBundle {
	pub mass: Mass,
	pub velocity: Velocity,
	pub impulse: Impulse,
	pub force: Force,
}
