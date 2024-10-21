use crate::prelude::*;
use bevy::prelude::*;



#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Debug, Default, PartialEq)]
pub struct PointsAnimation {
	pub wrap: bool,
	pub points: Vec<Vec3>,
}

impl Default for PointsAnimation {
	fn default() -> Self {
		Self {
			points: vec![-Vec3::X, Vec3::X],
			wrap: false,
		}
	}
}

impl PointsAnimation {
	/// create a new path that is open (i.e. not a loop)
	pub fn new_open(points: Vec<Vec3>) -> Self {
		Self {
			points,
			wrap: false,
		}
	}
	
	/// create a new path that is closed (i.e. a loop)
	pub fn new_closed(points: Vec<Vec3>) -> Self {
		Self {
			points,
			wrap: true,
		}
	}

	pub fn num_segments(&self) -> usize {
		if self.wrap {
			self.points.len()
		} else {
			self.points.len() - 1
		}
	}
}

impl ProceduralAnimationStrategy for PointsAnimation {
	fn fraction_to_pos(&self, t: f32) -> Vec3 {
		let t = t.clamp(0.0, 1.0);
		let t = t * self.num_segments() as f32;
		let segment = t.floor() as usize;
		let t = t - segment as f32;
		let i1 = segment;
		let i2 = (segment + 1) % self.points.len();
		self.points[i1].lerp(self.points[i2], t)
	}

	fn total_length(&self) -> f32 {
		let mut length = 0.0;
		for i in 0..self.num_segments() {
			let i1 = i;
			let i2 = (i + 1) % self.points.len();
			length += self.points[i1].distance(self.points[i2]);
		}
		length
	}
}
