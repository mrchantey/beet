use bevy::prelude::*;
use extend::ext;

#[ext]
pub impl Quat {
	//TODO forward faces back
	fn forward(&self) -> Vec3 { *self * Vec3::Z }

	fn from_up() -> Quat { Quat::look_at(Vec3::Y) }
	fn from_down() -> Quat { Quat::look_at(-Vec3::Y) }
	fn from_right() -> Quat { Quat::look_at(Vec3::X) }
	fn from_left() -> Quat { Quat::look_at(-Vec3::X) }
	fn from_forward() -> Quat { Quat::look_at(Vec3::Z) }
	fn from_back() -> Quat { Quat::look_at(-Vec3::Z) }

	fn look_at_with_up(target: Vec3, up: Vec3) -> Quat {
		let mat = Mat4::look_at_rh(target, Vec3::ZERO, up).inverse();
		Quat::from_mat4(&mat)
	}

	fn look_at(target: Vec3) -> Quat {
		let up = if target.x == 0. && target.z == 0. {
			-Vec3::Z
		} else {
			Vec3::Y
		};
		Self::look_at_with_up(target, up)
	}

	fn look_away(target: Vec3) -> Quat { Self::look_at(target * -1.) }


	//from threejs
	fn rotate_towards(&mut self, rhs: Quat, rad_step: f32) -> &Quat {
		let angle = self.angle_between(rhs);
		if angle == 0. {
			return self;
		};
		let t = f32::min(1., rad_step / angle);
		self.clone_from(&self.slerp(rhs, t));
		return self;
	}

	fn euler_xyz(&self) -> Vec3 {
		let (x, y, z) = self.to_euler(EulerRot::XYZ);
		Vec3::new(x, y, z)
	}
}


#[cfg(test)]
mod test {
	use bevy::prelude::*;
	use crate::prelude::*;
	use sweet_test::prelude::*;

	#[test]
	fn works() {
		let v = Quat::from_right();
		expect(v.forward().x).to_be_close_to(1.);
		let v = Quat::from_left();
		expect(v.forward().x).to_be_close_to(-1.);
		let v = Quat::from_up();
		expect(v.forward().y).to_be_close_to(1.);
		let v = Quat::from_down();
		expect(v.forward().y).to_be_close_to(-1.);
		let v = Quat::from_forward();
		expect(v.forward().z).to_be_close_to(1.);
		let v = Quat::from_back();
		expect(v.forward().z).to_be_close_to(-1.);
	}
}
