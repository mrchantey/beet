use bevy::prelude::*;

pub const DEFAULT_ULTRASOUND_MAX_DEPTH: f32 = 2.0;

#[derive(
	Debug, Default, Clone, Deref, DerefMut, Component, PartialEq, Reflect,
)]
pub struct DepthValue(pub Option<f32>);

impl DepthValue {
	pub fn new(value: f32) -> Self { Self(Some(value)) }
}
