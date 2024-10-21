use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Debug, Default, PartialEq)]
pub struct CircleAnimation {
	pub radius: f32,
	pub position: Vec3,
	pub rotation: Quat,
}

impl Default for CircleAnimation {
	fn default() -> Self {
		Self {
			radius: 0.5,
			position: Vec3::ZERO,
			rotation: Quat::IDENTITY,
		}
	}
}


impl ProceduralAnimationStrategy for CircleAnimation {
	/// C = 2πr
	fn total_length(&self) -> f32 { std::f32::consts::PI * 2.0 * self.radius }

	fn fraction_to_pos(&self, t: f32) -> Vec3 {
		let angle = t * std::f32::consts::PI * 2.0;
		let x = self.rotation * Vec3::X * angle.cos() * self.radius;
		let y = self.rotation * Vec3::Y * angle.sin() * self.radius;
		self.position + x + y
	}
}
