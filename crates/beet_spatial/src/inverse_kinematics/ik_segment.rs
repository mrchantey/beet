use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::FRAC_PI_4;
use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, Reflect)]
pub struct IkSegment {
	pub len: f32,
	pub min_angle: f32,
	pub max_angle: f32,
}


impl Default for IkSegment {
	fn default() -> Self {
		Self {
			len: 1.0,
			min_angle: -PI,
			max_angle: PI,
		}
	}
}

impl IkSegment {
	/// An IK segment with a length of 1.0 and a full rotation range.
	pub const DEG_360: Self = Self {
		len: 1.0,
		min_angle: -PI,
		max_angle: PI,
	};

	/// An IK segment with a length of 1.0 and a total .
	pub const DEG_180: Self = Self {
		len: 1.0,
		min_angle: -FRAC_PI_2,
		max_angle: FRAC_PI_2,
	};

	pub const DEG_90: Self = Self {
		len: 1.0,
		min_angle: -FRAC_PI_4,
		max_angle: FRAC_PI_4,
	};


	pub fn with_len(self, len: f32) -> Self { Self { len, ..self } }
	pub fn with_angle(self, angle: f32) -> Self {
		Self {
			min_angle: -angle,
			max_angle: angle,
			..self
		}
	}
}
