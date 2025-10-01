use crate::prelude::*;
use beet_core::prelude::*;
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::FRAC_PI_4;
use std::f32::consts::PI;

/// An individual segment, ie forearm, upper arm, of a kinematic chain.
#[derive(Debug, Clone, Copy, Reflect)]
pub struct IkSegment {
	/// The length of the segment.
	pub len: f32,
	/// The minimum angle in radians the segment can rotate.
	pub min_angle: Radians,
	/// The maximum angle in radians the segment can rotate.
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

	/// An IK segment with a length of 1.0 and a rotation range of -90 to 90 degrees.
	pub const DEG_180: Self = Self {
		len: 1.0,
		min_angle: -FRAC_PI_2,
		max_angle: FRAC_PI_2,
	};

	/// An IK segment with a length of 1.0 and a rotation range of -45 to 45 degrees.
	pub const DEG_90: Self = Self {
		len: 1.0,
		min_angle: -FRAC_PI_4,
		max_angle: FRAC_PI_4,
	};

	/// Create a new segment with the given length.
	pub fn with_len(self, len: f32) -> Self { Self { len, ..self } }
	/// Create a new segment with the given angle range.
	/// A provided angle of 45 will result in a range of -45 to 45 degrees.
	pub fn with_angle(self, angle: f32) -> Self {
		Self {
			min_angle: -angle,
			max_angle: angle,
			..self
		}
	}
}
