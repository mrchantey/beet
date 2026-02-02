//! Extension methods for Bevy's [`Quat`].

use crate::prelude::*;
use extend::ext;

/// Extension trait adding utility methods to [`Quat`].
#[ext]
pub impl Quat {
	/// Returns the forward direction (-Z) vector for this rotation.
	fn forward(&self) -> Vec3 { *self * -Vec3::Z }

	/// Creates a rotation facing up (+Y).
	fn from_up() -> Quat { Quat::look_at(Vec3::Y) }
	/// Creates a rotation facing down (-Y).
	fn from_down() -> Quat { Quat::look_at(-Vec3::Y) }
	/// Creates a rotation facing right (+X).
	fn from_right() -> Quat { Quat::look_at(Vec3::X) }
	/// Creates a rotation facing left (-X).
	fn from_left() -> Quat { Quat::look_at(-Vec3::X) }
	/// Creates a rotation facing forward (-Z).
	fn from_forward() -> Quat { Quat::look_at(-Vec3::Z) }
	/// Creates a rotation facing back (+Z).
	fn from_back() -> Quat { Quat::look_at(Vec3::Z) }

	/// Creates a rotation looking at the target with the specified up vector.
	fn look_at_with_up(target: Vec3, up: Vec3) -> Quat {
		let mat = Mat4::look_at_rh(Vec3::ZERO, target, up).inverse();
		Quat::from_mat4(&mat)
	}

	/// Creates a rotation looking at the target.
	fn look_at(target: Vec3) -> Quat {
		let up = if target.x == 0. && target.z == 0. {
			-Vec3::Z
		} else {
			Vec3::Y
		};
		Self::look_at_with_up(target, up)
	}

	/// Creates a rotation looking away from the target.
	fn look_away(target: Vec3) -> Quat { Self::look_at(target * -1.) }

	/// Rotates towards another quaternion by at most `rad_step` radians.
	fn rotate_towards(&mut self, rhs: Quat, rad_step: f32) -> &Quat {
		let angle = self.angle_between(rhs);
		if angle == 0. {
			return self;
		};
		let t = f32::min(1., rad_step / angle);
		self.clone_from(&self.slerp(rhs, t));
		return self;
	}

	/// Returns the Euler angles (XYZ order) as a [`Vec3`].
	fn euler_xyz(&self) -> Vec3 {
		let (x, y, z) = self.to_euler(EulerRot::XYZ);
		Vec3::new(x, y, z)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn works() {
		let quat = Quat::from_right();
		quat.forward().x.xpect_close(1.);
		let quat = Quat::from_left();
		quat.forward().x.xpect_close(-1.);
		let quat = Quat::from_up();
		quat.forward().y.xpect_close(1.);
		let quat = Quat::from_down();
		quat.forward().y.xpect_close(-1.);
		let quat = Quat::from_forward();
		quat.forward().z.xpect_close(-1.);
		let quat = Quat::from_back();
		quat.forward().z.xpect_close(1.);
	}
}
