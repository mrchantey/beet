use crate::prelude::*;
use bevy::prelude::*;
use extend::ext;

#[ext]
pub impl Transform {
	fn flat_y(&self) -> Vec3 { Vec3::Y }
	fn flat_x(&self) -> Vec3 {
		let mut vec: Vec3 = self.local_x().into();
		vec.y = 0.;
		vec.normalize_or_zero()
	}
	fn flat_z(&self) -> Vec3 {
		let mut vec: Vec3 = self.local_z().into();
		vec.y = 0.;
		vec.normalize_or_zero()
	}

	fn from_position(v: Vec3) -> Self { Self::from_translation(v) }

	fn with_position(&self, v: Vec3) -> Self { self.with_translation(v) }

	fn from_position_x(x: f32) -> Self {
		Self::from_translation(Vec3::new(x, 0., 0.))
	}
	fn from_position_y(y: f32) -> Self {
		Self::from_translation(Vec3::new(0., y, 0.))
	}
	fn from_position_z(z: f32) -> Self {
		Self::from_translation(Vec3::new(0., 0., z))
	}

	fn with_position_x(self, x: f32) -> Self {
		self.with_translation(Vec3::new(x, 0., 0.))
	}
	fn with_position_y(self, y: f32) -> Self {
		self.with_translation(Vec3::new(0., y, 0.))
	}
	fn with_position_z(self, z: f32) -> Self {
		self.with_translation(Vec3::new(0., 0., z))
	}
	fn from_rotation_x(x: f32) -> Self {
		Self::from_rotation(Quat::from_rotation_x(x))
	}
	fn from_rotation_y(y: f32) -> Self {
		Self::from_rotation(Quat::from_rotation_y(y))
	}
	fn from_rotation_z(z: f32) -> Self {
		Self::from_rotation(Quat::from_rotation_z(z))
	}

	fn with_rotation_x(self, x: f32) -> Self {
		self.with_rotation(Quat::from_rotation_x(x))
	}
	fn with_rotation_y(self, y: f32) -> Self {
		self.with_rotation(Quat::from_rotation_y(y))
	}
	fn with_rotation_z(self, z: f32) -> Self {
		self.with_rotation(Quat::from_rotation_z(z))
	}

	fn from_scale_xyz(x: f32, y: f32, z: f32) -> Self {
		Self::from_scale(Vec3::new(x, y, z))
	}
	fn with_scale_xyz(self, x: f32, y: f32, z: f32) -> Self {
		self.with_scale(Vec3::new(x, y, z))
	}
	fn from_scale_x(x: f32) -> Self { Self::from_scale(Vec3::new(x, 1., 1.)) }
	fn from_scale_y(y: f32) -> Self { Self::from_scale(Vec3::new(1., y, 1.)) }
	fn from_scale_z(z: f32) -> Self { Self::from_scale(Vec3::new(1., 1., z)) }

	fn with_scale_x(self, x: f32) -> Self {
		self.with_scale(Vec3::new(x, 1., 1.))
	}
	fn with_scale_y(self, y: f32) -> Self {
		self.with_scale(Vec3::new(1., y, 1.))
	}
	fn with_scale_z(self, z: f32) -> Self {
		self.with_scale(Vec3::new(1., 1., z))
	}
	fn with_scale_value(self, v: f32) -> Self {
		self.with_scale(Vec3::splat(v))
	}

	fn translate_x(&mut self, val: f32) {
		self.translation += self.local_x() * val;
	}
	fn translate_y(&mut self, val: f32) {
		self.translation += self.local_y() * val;
	}
	fn translate_z(&mut self, val: f32) {
		self.translation += self.local_z() * val;
	}
	fn translate_flat_x(&mut self, val: f32) {
		self.translation += self.flat_x() * val;
	}
	fn translate_flat_y(&mut self, val: f32) {
		self.translation += Vec3::Y * val;
	}
	fn translate_flat_z(&mut self, val: f32) {
		self.translation += self.flat_z() * val;
	}

	fn translate_local(&mut self, val: Vec3) {
		let translation = self.local_x() * val.x
			+ self.local_y() * val.y
			+ self.local_z() * val.z;
		self.translation += translation;
	}



	fn look_away(&mut self, target: Vec3, up: Vec3) {
		let forward = Vec3::normalize(target - self.translation);
		let right = up.cross(forward).normalize();
		let up = forward.cross(right);
		self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, forward));
	}
	fn looking_away(mut self, target: Vec3, up: Vec3) -> Self {
		self.look_away(target, up);
		self
	}
	fn from_pose(&mut self, pose: &Pose) {
		self.translation = pose.position;
		self.rotation = pose.rotation;
	}
}
