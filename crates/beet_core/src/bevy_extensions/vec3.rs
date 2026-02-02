//! Extension methods for Bevy's [`Vec3`].

use crate::prelude::*;
use extend::ext;

/// Extension trait adding utility methods to [`Vec3`].
#[ext]
pub impl Vec3 {
	/// Unit vector pointing right (+X).
	const RIGHT: Vec3 = Vec3::new(1., 0., 0.);
	/// Unit vector pointing left (-X).
	const LEFT: Vec3 = Vec3::new(-1., 0., 0.);
	/// Unit vector pointing up (+Y).
	const UP: Vec3 = Vec3::new(0., 1., 0.);
	/// Unit vector pointing down (-Y).
	const DOWN: Vec3 = Vec3::new(0., -1., 0.);
	/// Unit vector pointing positive Z (+Z).
	const Z: Vec3 = Vec3::new(0., 0., 1.);
	/// Unit vector pointing negative Z (-Z).
	const Z_NEG: Vec3 = Vec3::new(0., 0., -1.);

	/// Creates a vector with only the X component set.
	fn from_x(x: f32) -> Self { Vec3::new(x, 0., 0.) }
	/// Creates a vector with only the Y component set.
	fn from_y(y: f32) -> Self { Vec3::new(0., y, 0.) }
	/// Creates a vector with only the Z component set.
	fn from_z(z: f32) -> Self { Vec3::new(0., 0., z) }
	/// Adds to the X component in place.
	fn add_x(&mut self, x: f32) -> &mut Self {
		self.x += x;
		self
	}
	/// Adds to the Y component in place.
	fn add_y(&mut self, y: f32) -> &mut Self {
		self.y += y;
		self
	}
	/// Adds to the Z component in place.
	fn add_z(&mut self, z: f32) -> &mut Self {
		self.z += z;
		self
	}
	/// Swaps the X and Y components in place.
	fn swap_xy(&mut self) -> &mut Self {
		let tmp = self.x;
		self.x = self.y;
		self.y = tmp;
		self
	}
	/// Swaps the X and Z components in place.
	fn swap_xz(&mut self) -> &mut Self {
		let tmp = self.x;
		self.x = self.z;
		self.z = tmp;
		self
	}
	/// Swaps the Y and Z components in place.
	fn swap_yz(&mut self) -> &mut Self {
		let tmp = self.z;
		self.z = self.y;
		self.y = tmp;
		self
	}
	/// Negates the X component in place.
	fn negate_x(&mut self) -> &mut Self {
		self.x = -self.x;
		self
	}
	/// Negates the Y component in place.
	fn negate_y(&mut self) -> &mut Self {
		self.y = -self.y;
		self
	}
	/// Negates the Z component in place.
	fn negate_z(&mut self) -> &mut Self {
		self.z = -self.z;
		self
	}

	#[cfg(feature = "rand")]
	/// Returns a random position inside a signed unit cube (-1 to 1 on each axis).
	fn random_in_cube_signed(rng: &mut impl rand::Rng) -> Self {
		Vec3::new(
			rng.random_range(-1.0..1.0),
			rng.random_range(-1.0..1.0),
			rng.random_range(-1.0..1.0),
		)
	}

	#[cfg(feature = "rand")]
	/// Returns a random position inside a unit cube (0 to 1 on each axis).
	fn random_in_cube(rng: &mut impl rand::Rng) -> Self {
		Vec3::new(
			rng.random_range(0.0..1.0),
			rng.random_range(0.0..1.0),
			rng.random_range(0.0..1.0),
		)
	}

	#[cfg(feature = "rand")]
	/// Returns a random position on the surface of a unit sphere.
	fn random_on_sphere(rng: &mut impl rand::Rng) -> Self {
		let theta = rng.random_range(0.0..std::f32::consts::TAU);
		let phi = rng.random_range(0.0..std::f32::consts::PI);
		Vec3::new(phi.sin() * theta.cos(), phi.sin() * theta.sin(), phi.cos())
	}

	#[cfg(feature = "rand")]
	/// Returns a random position inside a unit sphere.
	fn random_in_sphere(rng: &mut impl rand::Rng) -> Self {
		let theta = rng.random_range(0.0..std::f32::consts::TAU);
		let phi = rng.random_range(0.0..std::f32::consts::PI);
		let r = rng.random_range(0.0f32..1.0).powf(1. / 3.);
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

	#[test]
	pub fn works() {
		let mut rng = rand::rng();
		for _ in 0..10 {
			let val = Vec3::random_in_cube(&mut rng);
			val.length().xpect_less_than(2.);
			// println!("random_in_cube: {val}");
		}
		for _ in 0..10 {
			let val = Vec3::random_on_sphere(&mut rng);
			val.length().xpect_close(1.);
			// println!("random_on_sphere: {val}");
		}
		for _ in 0..10 {
			let val = Vec3::random_in_sphere(&mut rng);
			val.length().xpect_less_than(1.);
			// println!("random_in_sphere: {val}");
		}
	}
}
