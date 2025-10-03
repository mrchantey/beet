use beet_core::prelude::*;
use std::ops::Range;


#[derive(Component, Reflect, Deref, DerefMut)]
#[reflect(Default, Component)]
pub struct StatRange(pub Range<f32>);

impl Default for StatRange {
	/// defaults to 0..1
	fn default() -> Self { Self(0.0..1.) }
}
