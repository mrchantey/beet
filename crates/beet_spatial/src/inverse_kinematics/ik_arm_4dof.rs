use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;
use std::f32::consts::TAU;

/// A type alias for radians as f32.
pub type Radians = f32;

/// Inverse Kinematics solver for a 2dof planar arm, for instance shoulder & elbow hinge joints.
#[derive(Debug, Clone, Copy, Reflect)]
pub struct IkArm4Dof {
	/// Set the offset from a Vec3::RIGHT direction in radians.
	pub base_offset_angle: Radians,
	/// The base segment (shoulder to elbow) of the arm.
	pub segment1: IkSegment,
	/// The second segment (elbow to wrist) of the arm.
	pub segment2: IkSegment,
	/// The third segment (wrist to fingertip) of the arm.
	pub segment3: IkSegment,
	/// The preferred style of the arm, overarm or underarm.
	pub arm_style: IkArmStyle,
}

impl Default for IkArm4Dof {
	fn default() -> Self {
		Self {
			base_offset_angle: 0.0,
			segment1: IkSegment::DEG_360,
			segment2: IkSegment::DEG_360,
			segment3: IkSegment::DEG_360.with_len(0.2),
			arm_style: default(),
		}
	}
}

/// The preferred style of the arm, overarm or underarm.
#[derive(Debug, Default, Clone, Copy, Reflect)]
pub enum IkArmStyle {
	/// The 'elbow' is generally above the 'wrist' joint.
	#[default]
	Overarm,
	/// The 'elbow' is generally below the 'wrist' joint.
	Underarm,
}

impl IkArm4Dof {
	/// Create a new 4DOF arm with the given segments and offset angle in radians.
	pub fn new(
		base_offset_angle: Radians,
		segment1: IkSegment,
		segment2: IkSegment,
		segment3: IkSegment,
	) -> Self {
		Self {
			base_offset_angle,
			segment1,
			segment2,
			segment3,
			arm_style: default(),
		}
	}

	/// The maximum reach (length) of the arm.
	pub fn reach(&self) -> f32 { self.segment1.len + self.segment2.len }

	/// Solve the inverse kinematics problem for a 4DOF arm with a base and three segments.
	pub fn solve4d(&self, delta_pos: Vec3) -> (f32, f32, f32, f32) {
		let angles = self.solve3d(delta_pos);
		let theta4 = TAU - angles.1 - angles.2;
		(angles.0, angles.1, angles.2, theta4)
	}


	/// Solve the inverse kinematics problem for a 3DOF arm with a base and two segments.
	/// Returns angles for the base, segment1 and segment2 in radians
	pub fn solve3d(&self, delta_pos: Vec3) -> (f32, f32, f32) {
		let delta_pos_flat = Vec2::new(delta_pos.x, delta_pos.z);
		let angle_base = f32::atan2(delta_pos_flat.y, delta_pos_flat.x);
		let hypotenuse = delta_pos_flat.length();
		let (angle_segment1, angle_segment2) =
			self.solve2d(Vec2::new(hypotenuse, delta_pos.y));
		(
			angle_base + self.base_offset_angle,
			angle_segment1,
			angle_segment2,
		)
	}

	/// Solve the inverse kinematics problem for a 2DOF arm using the Law of Cosines technique.
	/// Returns angles in radians
	pub fn solve2d(&self, mut target: Vec2) -> (f32, f32) {
		let (l1, l2) = (self.segment1.len, self.segment2.len);
		let reach = self.reach();

		// if the target is not reachable, clamp the target to the maximum reach
		if target.length_squared() > reach.powi(2) {
			target = target.normalize_or_zero() * (reach * 0.999);
		}

		// in the case of negative x, flip the target to positive x
		let is_neg = target.x < 0.0;
		if is_neg {
			target.x = -target.x;
			target.y = -target.y;
		};


		// Calculate angle for segment 2 using the Law of Cosines
		let cos_angle2 =
			(target.x.powi(2) + target.y.powi(2) - l1.powi(2) - l2.powi(2))
				/ (2.0 * l1 * l2);
		let angle2 = match self.arm_style {
			IkArmStyle::Overarm => -cos_angle2.acos(),
			IkArmStyle::Underarm => cos_angle2.acos(),
		}
		.clamp(self.segment2.min_angle, self.segment2.max_angle);

		// Calculate angle for segment 1 using the Law of Cosines and adjust for target position
		let k1 = l1 + l2 * angle2.cos();
		let k2 = l2 * angle2.sin();
		let angle1 = (target.y.atan2(target.x) - k2.atan2(k1))
			.clamp(self.segment1.min_angle, self.segment1.max_angle);

		if is_neg {
			(angle1 + PI, angle2)
		} else {
			(angle1, angle2)
		}
	}
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use std::f32::consts::PI;

	#[test]
	fn test_reachable_target() {
		let ik_solver = IkArm4Dof::default();

		let target = Vec2::new(1.0, 1.0);
		let (angle1, angle2) = ik_solver.solve2d(target);

		// Assert the angles are within reasonable bounds for this reachable target
		assert!((angle1).abs() <= PI);
		assert!((angle2).abs() <= PI);
	}

	#[test]
	fn test_unreachable_target_too_far() {
		let ik_solver = IkArm4Dof::default();

		let target = Vec2::new(3.0, 3.0);
		let (angle1, angle2) = ik_solver.solve2d(target);

		assert_eq!((angle1, angle2), (0.8301215, -0.089446634));
	}
}
