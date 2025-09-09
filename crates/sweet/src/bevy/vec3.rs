use crate::prelude::*;
use bevy::prelude::*;

impl CloseTo for Vec3 {
	fn default_delta() -> Self {
		Vec3::new(DEFAULT_DELTA_F32, DEFAULT_DELTA_F32, DEFAULT_DELTA_F32)
	}
	fn is_close_with_delta(&self, b: &Self, delta: &Self) -> bool {
		is_close_f32(self.x, b.x, delta.x)
			&& is_close_f32(self.y, b.y, delta.y)
			&& is_close_f32(self.z, b.z, delta.z)
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
		Vec3::ZERO.xpect_close(Vec3::ZERO);
		Vec3::ZERO.xnot().xpect_close(Vec3::ONE);
		Foo(Vec3::ZERO).xpect_close(Vec3::ZERO);
		Foo(Vec3::ZERO)
			.0
			.xnot()
			.xpect_close(Vec3::new(0.2, 0.2, 0.2));
	}
}
