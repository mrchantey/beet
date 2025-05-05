use bevy::prelude::*;
use extend::ext;

#[ext]
pub impl Vec3 {
	const RIGHT: Vec3 = Vec3::new(1., 0., 0.);
	const LEFT: Vec3 = Vec3::new(-1., 0., 0.);
	const UP: Vec3 = Vec3::new(0., 1., 0.);
	const DOWN: Vec3 = Vec3::new(0., -1., 0.);
	const Z: Vec3 = Vec3::new(0., 0., 1.);
	const Z_NEG: Vec3 = Vec3::new(0., 0., -1.);

	fn from_x(x: f32) -> Self { Vec3::new(x, 0., 0.) }
	fn from_y(y: f32) -> Self { Vec3::new(0., y, 0.) }
	fn from_z(z: f32) -> Self { Vec3::new(0., 0., z) }
	fn add_x(&mut self, x: f32) -> &mut Self {
		self.x += x;
		self
	}
	fn add_y(&mut self, y: f32) -> &mut Self {
		self.y += y;
		self
	}
	fn add_z(&mut self, z: f32) -> &mut Self {
		self.z += z;
		self
	}
	fn swap_xy(&mut self) -> &mut Self {
		let tmp = self.x;
		self.x = self.y;
		self.y = tmp;
		self
	}
	fn swap_xz(&mut self) -> &mut Self {
		let tmp = self.x;
		self.x = self.z;
		self.z = tmp;
		self
	}
	fn swap_yz(&mut self) -> &mut Self {
		let tmp = self.z;
		self.z = self.y;
		self.y = tmp;
		self
	}
	fn negate_x(&mut self) -> &mut Self {
		self.x = -self.x;
		self
	}
	fn negate_y(&mut self) -> &mut Self {
		self.y = -self.y;
		self
	}
	fn negate_z(&mut self) -> &mut Self {
		self.z = -self.z;
		self
	}

	#[cfg(feature = "rand")]
	/// Random position inside a unit cube (0, 1)
	fn random_in_cube_signed(rng: &mut impl rand::Rng) -> Self {
		Vec3::new(
			rng.gen_range(-1.0..1.0),
			rng.gen_range(-1.0..1.0),
			rng.gen_range(-1.0..1.0),
		)
	}

	#[cfg(feature = "rand")]
	fn random_in_cube(rng: &mut impl rand::Rng) -> Self {
		Vec3::new(
			rng.gen_range(0.0..1.0),
			rng.gen_range(0.0..1.0),
			rng.gen_range(0.0..1.0),
		)
	}

	#[cfg(feature = "rand")]
	/// Random position on a unit sphere
	fn random_on_sphere(rng: &mut impl rand::Rng) -> Self {
		let theta = rng.gen_range(0.0..std::f32::consts::TAU);
		let phi = rng.gen_range(0.0..std::f32::consts::PI);
		Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos())
	}

	#[cfg(feature = "rand")]
	/// Random position inside a unit sphere
	fn random_in_sphere(rng: &mut impl rand::Rng) -> Self {
		let theta = rng.gen_range(0.0..std::f32::consts::TAU);
		let phi = rng.gen_range(0.0..std::f32::consts::PI);
		let r = rng.gen_range(0.0f32..1.0).powf(1. / 3.);
		Vec3::new(
			r * phi.sin() * theta.cos(),
			r * phi.sin() * theta.sin(),
			r * phi.cos(),
		)
	}
}


#[cfg(feature = "rand")]
#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet_test::prelude::*;

	#[test]
	pub fn works() {
		let mut rng = rand::thread_rng();
		for _ in 0..10 {
			let val = Vec3::random_in_cube(&mut rng);
			expect(val.length()).to_be_less_than(2.);
			// println!("random_in_cube: {val}");
		}
		for _ in 0..10 {
			let val = Vec3::random_on_sphere(&mut rng);
			expect(val.length()).to_be_close_to(1.);
			// println!("random_on_sphere: {val}");
		}
		for _ in 0..10 {
			let val = Vec3::random_in_sphere(&mut rng);
			expect(val.length()).to_be_less_than(1.);
			// println!("random_in_sphere: {val}");
		}
	}
}
