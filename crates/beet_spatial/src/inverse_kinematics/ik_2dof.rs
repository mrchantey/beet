use crate::prelude::*;
use bevy::prelude::*;
use std::f32::consts::PI;

/// Inverse Kinematics solver for a 2dof planar arm, for instance shoulder & elbow hinge joints.
#[derive(Debug, Default, Clone, Copy, Reflect)]
pub struct Ik2Dof {
	pub segment1: IkSegment,
	pub segment2: IkSegment,
	pub arm_style: IkArmStyle,
}

#[derive(Debug, Default, Clone, Copy, Reflect)]
pub enum IkArmStyle {
	#[default]
	Overarm,
	Underarm,
}


pub enum Ik2DofSolution {
	/// The target is reachable and the solution is valid.
	Reachable(f32, f32),
	Stretched(f32, f32),
	/// The target angle cannot be reached, but this is as close as it can get.
	Invalid(f32, f32),
}


impl Ik2Dof {
	pub fn new(segment1: IkSegment, segment2: IkSegment) -> Self {
		Self {
			segment1,
			segment2,
			arm_style: default(),
		}
	}

	pub fn reach(&self) -> f32 { self.segment1.len + self.segment2.len }

	/// Solve the inverse kinematics problem for a 2DOF arm using the Law of Cosines technique.
	/// Returns angles in radians
	pub fn solve(&self, mut target: Vec2) -> (f32, f32) {
		let (l1, l2) = (self.segment1.len, self.segment2.len);
		let reach = self.reach();

		// if the target is not reachable, clamp the target to the maximum reach
		if target.length_squared() > reach.powi(2) {
			target = target.normalize_or_zero() * (reach - f32::EPSILON);
		}

		// in the case of negative x, flip the target to positive x
		let is_neg = target.x < 0.0;
		if target.x < 0. {
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
mod tests {
	use crate::prelude::*;
	use bevy::prelude::*;
	use std::f32::consts::PI;

	#[test]
	fn test_reachable_target() {
		let ik_solver = Ik2Dof::default();

		let target = Vec2::new(1.0, 1.0);
		let (angle1, angle2) = ik_solver.solve(target);

		// Assert the angles are within reasonable bounds for this reachable target
		assert!((angle1).abs() <= PI);
		assert!((angle2).abs() <= PI);
	}

	#[test]
	fn test_unreachable_target_too_far() {
		let ik_solver = Ik2Dof::default();

		let target = Vec2::new(3.0, 3.0);
		let (angle1, angle2) = ik_solver.solve(target);

		// Since the target is unreachable, check if it returns the neutral position or an indicator value
		assert_eq!((angle1, angle2), (0.0, 0.0));
	}

	// #[test]
	// fn test_unreachable_target_too_close() {
	// 	let ik_solver = Ik2Dof::default();

	// 	let target = Vec2::new(0.1, 0.1);
	// 	let (angle1, angle2) = ik_solver.solve(target);

	// 	// Since the target is unreachable due to being too close, it should also return neutral position
	// 	assert_eq!((angle1, angle2), (0.0, 0.0));
	// }

	#[test]
	fn test_within_joint_limits() {
		let segment1 = IkSegment {
			len: 1.0,
			min_angle: -std::f32::consts::FRAC_PI_2, // -90 degrees
			max_angle: std::f32::consts::FRAC_PI_2,  // 90 degrees
		};
		let segment2 = IkSegment {
			len: 1.0,
			min_angle: -std::f32::consts::FRAC_PI_2, // -90 degrees
			max_angle: std::f32::consts::FRAC_PI_2,  // 90 degrees
		};
		let ik_solver = Ik2Dof::new(segment1, segment2);

		let target = Vec2::new(1.0, 0.0);
		let (angle1, angle2) = ik_solver.solve(target);

		// Assert the angles are within the specified joint limits
		assert!(
			angle1 >= -std::f32::consts::FRAC_PI_2
				&& angle1 <= std::f32::consts::FRAC_PI_2
		);
		assert!(
			angle2 >= -std::f32::consts::FRAC_PI_2
				&& angle2 <= std::f32::consts::FRAC_PI_2
		);
	}
}
