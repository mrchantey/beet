use crate::prelude::*;
use bevy::prelude::*;

pub trait ProceduralAnimationStrategy {
	/// For a given value between 0 (inclusive) and 1 (not inclusive), return a position.
	///
	/// # Panics
	/// May panic if `t` is:
	/// - less than zero
	/// - equal to or greater than 1
	fn fraction_to_pos(&self, t: f32) -> Vec3;
	/// Sum of the lengths of all segments.
	/// For some shapes this may be an approximation.
	fn total_length(&self) -> f32;
}

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Debug, Default, PartialEq)]
pub enum ProceduralAnimationShape {
	Circle(CircleAnimation),
	Points(PointsAnimation),
}

impl Default for ProceduralAnimationShape {
	fn default() -> Self { Self::Circle(CircleAnimation::default()) }
}



impl ProceduralAnimationStrategy for ProceduralAnimationShape {
	fn fraction_to_pos(&self, t: f32) -> Vec3 {
		match self {
			ProceduralAnimationShape::Circle(circle) => {
				circle.fraction_to_pos(t)
			}
			ProceduralAnimationShape::Points(points) => {
				points.fraction_to_pos(t)
			}
		}
	}
	fn total_length(&self) -> f32 {
		match self {
			ProceduralAnimationShape::Circle(circle) => circle.total_length(),
			ProceduralAnimationShape::Points(points) => points.total_length(),
		}
	}
}
