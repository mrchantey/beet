//! Extension methods for Bevy's [`Transform`].

use crate::prelude::*;
use extend::ext;

/// Extension trait adding utility methods to [`Transform`].
#[ext]
pub impl Transform {
	/// Returns the Y-axis unit vector (always up).
	fn flat_y(&self) -> Vec3 { Vec3::Y }
	/// Returns the local X-axis projected onto the XZ plane and normalized.
	fn flat_x(&self) -> Vec3 {
		let mut vec: Vec3 = self.local_x().into();
		vec.y = 0.;
		vec.normalize_or_zero()
	}
	/// Returns the local Z-axis projected onto the XZ plane and normalized.
	fn flat_z(&self) -> Vec3 {
		let mut vec: Vec3 = self.local_z().into();
		vec.y = 0.;
		vec.normalize_or_zero()
	}

	/// Creates a transform with the given position (alias for `from_translation`).
	fn from_position(v: Vec3) -> Self { Self::from_translation(v) }

	/// Returns a copy with the given position (alias for `with_translation`).
	fn with_position(&self, v: Vec3) -> Self { self.with_translation(v) }

	/// Creates a transform positioned along the X-axis.
	fn from_position_x(x: f32) -> Self {
		Self::from_translation(Vec3::new(x, 0., 0.))
	}
	/// Creates a transform positioned along the Y-axis.
	fn from_position_y(y: f32) -> Self {
		Self::from_translation(Vec3::new(0., y, 0.))
	}
	/// Creates a transform positioned along the Z-axis.
	fn from_position_z(z: f32) -> Self {
		Self::from_translation(Vec3::new(0., 0., z))
	}

	/// Returns a copy positioned along the X-axis.
	fn with_position_x(self, x: f32) -> Self {
		self.with_translation(Vec3::new(x, 0., 0.))
	}
	/// Returns a copy positioned along the Y-axis.
	fn with_position_y(self, y: f32) -> Self {
		self.with_translation(Vec3::new(0., y, 0.))
	}
	/// Returns a copy positioned along the Z-axis.
	fn with_position_z(self, z: f32) -> Self {
		self.with_translation(Vec3::new(0., 0., z))
	}
	/// Creates a transform rotated around the X-axis.
	fn from_rotation_x(x: f32) -> Self {
		Self::from_rotation(Quat::from_rotation_x(x))
	}
	/// Creates a transform rotated around the Y-axis.
	fn from_rotation_y(y: f32) -> Self {
		Self::from_rotation(Quat::from_rotation_y(y))
	}
	/// Creates a transform rotated around the Z-axis.
	fn from_rotation_z(z: f32) -> Self {
		Self::from_rotation(Quat::from_rotation_z(z))
	}

	/// Returns a copy rotated around the X-axis.
	fn with_rotation_x(self, x: f32) -> Self {
		self.with_rotation(Quat::from_rotation_x(x))
	}
	/// Returns a copy rotated around the Y-axis.
	fn with_rotation_y(self, y: f32) -> Self {
		self.with_rotation(Quat::from_rotation_y(y))
	}
	/// Returns a copy rotated around the Z-axis.
	fn with_rotation_z(self, z: f32) -> Self {
		self.with_rotation(Quat::from_rotation_z(z))
	}

	/// Creates a transform with non-uniform scale.
	fn from_scale_xyz(x: f32, y: f32, z: f32) -> Self {
		Self::from_scale(Vec3::new(x, y, z))
	}
	/// Returns a copy with non-uniform scale.
	fn with_scale_xyz(self, x: f32, y: f32, z: f32) -> Self {
		self.with_scale(Vec3::new(x, y, z))
	}
	/// Creates a transform scaled along the X-axis.
	fn from_scale_x(x: f32) -> Self { Self::from_scale(Vec3::new(x, 1., 1.)) }
	/// Creates a transform scaled along the Y-axis.
	fn from_scale_y(y: f32) -> Self { Self::from_scale(Vec3::new(1., y, 1.)) }
	/// Creates a transform scaled along the Z-axis.
	fn from_scale_z(z: f32) -> Self { Self::from_scale(Vec3::new(1., 1., z)) }

	/// Returns a copy scaled along the X-axis.
	fn with_scale_x(self, x: f32) -> Self {
		self.with_scale(Vec3::new(x, 1., 1.))
	}
	/// Returns a copy scaled along the Y-axis.
	fn with_scale_y(self, y: f32) -> Self {
		self.with_scale(Vec3::new(1., y, 1.))
	}
	/// Returns a copy scaled along the Z-axis.
	fn with_scale_z(self, z: f32) -> Self {
		self.with_scale(Vec3::new(1., 1., z))
	}
	/// Returns a copy with uniform scale.
	fn with_scale_value(self, v: f32) -> Self {
		self.with_scale(Vec3::splat(v))
	}

	/// Translates along the local X-axis.
	fn translate_x(&mut self, val: f32) {
		self.translation += self.local_x() * val;
	}
	/// Translates along the local Y-axis.
	fn translate_y(&mut self, val: f32) {
		self.translation += self.local_y() * val;
	}
	/// Translates along the local Z-axis.
	fn translate_z(&mut self, val: f32) {
		self.translation += self.local_z() * val;
	}
	/// Translates along the local X-axis projected onto the XZ plane.
	fn translate_flat_x(&mut self, val: f32) {
		self.translation += self.flat_x() * val;
	}
	/// Translates along the world Y-axis.
	fn translate_flat_y(&mut self, val: f32) {
		self.translation += Vec3::Y * val;
	}
	/// Translates along the local Z-axis projected onto the XZ plane.
	fn translate_flat_z(&mut self, val: f32) {
		self.translation += self.flat_z() * val;
	}

	/// Translates in local space by the given vector.
	fn translate_local(&mut self, val: Vec3) {
		let translation = self.local_x() * val.x
			+ self.local_y() * val.y
			+ self.local_z() * val.z;
		self.translation += translation;
	}



	/// Rotates to look away from the target (opposite of `look_at`).
	fn look_away(&mut self, target: Vec3, up: Vec3) {
		let forward = Vec3::normalize(target - self.translation);
		let right = up.cross(forward).normalize();
		let up = forward.cross(right);
		self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
	}
	/// Returns a copy looking away from the target.
	fn looking_away(mut self, target: Vec3, up: Vec3) -> Self {
		self.look_away(target, up);
		self
	}
	/// Sets position and rotation from a [`Pose`].
	fn from_pose(&mut self, pose: &Pose) {
		self.translation = pose.position;
		self.rotation = pose.rotation;
	}
}
