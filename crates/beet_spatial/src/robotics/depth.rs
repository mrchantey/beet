use beet_core::prelude::*;


/// The default maximum depth for ultrasound sensors,
/// this effect the time of flight cutoff, ie how long
/// to wait for a bounce before giving up.
pub const DEFAULT_ULTRASOUND_MAX_DEPTH: f32 = 2.0;


/// Represents the current reading of a depth sensor,
/// if this is none the last reading was invalid, usually
/// because the sensor is out of range.
#[derive(
	Debug, Default, Clone, Deref, DerefMut, Component, PartialEq, Reflect,
)]
pub struct DepthValue(pub Option<f32>);

impl DepthValue {
	/// Create a new depth value with the given distance.
	pub fn new(dist: f32) -> Self { Self(Some(dist)) }
}
