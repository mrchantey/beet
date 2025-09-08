use crate::prelude::*;
use bevy::prelude::*;

impl CloseTo for Vec3 {
	fn default_epsilon() -> Self {
		Vec3::new(
			DEFAULT_EPSILON_F32,
			DEFAULT_EPSILON_F32,
			DEFAULT_EPSILON_F32,
		)
	}
	fn is_close_with_epsilon(a: Self, b: Self, epsilon: Self) -> bool {
		is_close_f32(a.x, b.x, epsilon.x)
			&& is_close_f32(a.y, b.y, epsilon.y)
			&& is_close_f32(a.z, b.z, epsilon.z)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[derive(Debug, Clone, Copy, PartialEq, Deref, Component)]
	struct Foo(pub Vec3);

	#[test]
	fn vec3() {
		Vec3::ZERO.xpect().to_be_close_to(Vec3::ZERO);
		Vec3::ZERO.xpect().not().to_be_close_to(Vec3::ONE);
		Foo(Vec3::ZERO).0.xpect().to_be_close_to(Vec3::ZERO);
		Foo(Vec3::ZERO)
			.0
			.xpect()
			.not()
			.to_be_close_to(Vec3::new(0.2, 0.2, 0.2));
	}
}
