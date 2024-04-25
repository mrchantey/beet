use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
/// Add to agents to set the parameters of separation, alignment and cohesion
pub struct GroupParams {
	pub separate_radius: f32,
	pub align_radius: f32,
	pub cohere_radius: f32,
}

impl Default for GroupParams {
	fn default() -> Self {
		Self {
			separate_radius: 0.2,
			align_radius: 0.5,
			cohere_radius: 0.5,
		}
	}
}

impl GroupParams {
	pub fn new(
		separate_radius: f32,
		align_radius: f32,
		cohere_radius: f32,
	) -> Self {
		Self {
			separate_radius,
			align_radius,
			cohere_radius,
		}
	}

	pub fn scaled_to(mut self, val: f32) -> Self {
		self.separate_radius *= val;
		self.align_radius *= val;
		self.cohere_radius *= val;
		self
	}
}
