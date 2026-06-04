//! A lightweight position and rotation representation.

use crate::prelude::*;

/// A lightweight struct holding position and rotation.
///
/// This is a simpler alternative to [`Transform`] when scale is not needed.
#[derive(Debug)]
pub struct Pose {
	/// The position in 3D space.
	pub position: Vec3,
	/// The rotation as a quaternion.
	pub rotation: Quat,
}

impl Default for Pose {
	fn default() -> Self {
		Pose {
			position: Vec3::default(),
			rotation: Quat::default(),
		}
	}
}


impl Pose {
	/// Creates a 2D [`Pose`] from a planar position and a yaw about the Z axis,
	/// the natural form for a ground robot's `(x, y, theta)` pose.
	pub fn from_xy_theta(x: f32, y: f32, theta: f32) -> Pose {
		Pose {
			position: Vec3::new(x, y, 0.0),
			rotation: Quat::from_rotation_z(theta),
		}
	}

	/// The yaw about the Z axis, the planar heading of a 2D [`Pose`].
	pub fn yaw(&self) -> f32 { self.rotation.to_euler(EulerRot::ZYX).0 }

	/// Creates a new [`Pose`] from the translation and rotation of a [`Transform`].
	pub fn from_transform(tran: &Transform) -> Pose {
		Pose {
			position: tran.translation,
			rotation: tran.rotation,
		}
	}
	/// Sets the position and rotation from a [`Transform`].
	pub fn set_from_transform(&mut self, tran: &Transform) {
		self.position = tran.translation;
		self.rotation = tran.rotation;
	}

	/// Linearly interpolates between two poses.
	pub fn lerp(a: &Pose, b: &Pose, t: f32) -> Pose {
		Pose {
			position: Vec3::lerp(a.position, b.position, t),
			rotation: Quat::slerp(a.rotation, b.rotation, t),
		}
	}
}
