use bevy::prelude::*;

pub trait ProceduralAnimationStrategy {
	/// For a given value between 0 (inclusive) and 1 (not inclusive), return a position.
	///
	/// # Panics
	/// May panic if `t` is:
	/// - less than zero
	/// - equal to or greater than 1
	fn get_position(&self, t: f32) -> Vec3;
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
	fn default() -> Self {
		Self::Circle(CircleAnimation::default())
	}
}



impl ProceduralAnimationStrategy for ProceduralAnimationShape {
	fn get_position(&self, t: f32) -> Vec3 {
		match self {
			ProceduralAnimationShape::Circle(circle) => circle.get_position(t),
			ProceduralAnimationShape::Points(points) => points.get_position(t),
		}
	}
	fn total_length(&self) -> f32 {
		match self {
			ProceduralAnimationShape::Circle(circle) => circle.total_length(),
			ProceduralAnimationShape::Points(points) => points.total_length(),
		}
	}
}


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

	fn get_position(&self, t: f32) -> Vec3 {
		let angle = t * std::f32::consts::PI * 2.0;
		let x = self.rotation * Vec3::X * angle.cos() * self.radius;
		let y = self.rotation * Vec3::Y * angle.sin() * self.radius;
		self.position + x + y
	}
}


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
	pub fn num_segments(&self) -> usize {
		if self.wrap {
			self.points.len()
		} else {
			self.points.len() - 1
		}
	}
}

impl ProceduralAnimationStrategy for PointsAnimation {
	fn get_position(&self, t: f32) -> Vec3 {
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
