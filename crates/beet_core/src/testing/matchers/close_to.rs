//! Floating-point approximate equality utilities.
//!
//! This module provides the [`CloseTo`] trait for comparing floating-point
//! values with a configurable tolerance, useful for testing numerical results.

use bevy::prelude::*;

/// Default delta tolerance for f32 comparisons.
pub const DEFAULT_DELTA_F32: f32 = 0.1;

/// Default delta tolerance for f64 comparisons.
pub const DEFAULT_DELTA_F64: f64 = 0.1;

/// Trait for approximate equality comparisons.
///
/// This is primarily used by [`xpect_close`](super::MatcherClose::xpect_close)
/// to compare floating-point values within a tolerance.
pub trait CloseTo: Sized {
	/// Returns the default delta tolerance for this type.
	fn default_delta() -> Self;

	/// Checks if two values are approximately equal within the given epsilon.
	fn is_close_with_delta(&self, b: &Self, epsilon: &Self) -> bool;

	/// Checks if two values are approximately equal using the default delta.
	fn is_close(&self, b: &Self) -> bool {
		Self::is_close_with_delta(self, b, &Self::default_delta())
	}
}

impl CloseTo for f32 {
	fn default_delta() -> Self { DEFAULT_DELTA_F32 }
	fn is_close_with_delta(&self, b: &Self, epsilon: &Self) -> bool {
		is_close_f32(*self, *b, *epsilon)
	}
}
impl CloseTo for f64 {
	fn default_delta() -> Self { DEFAULT_DELTA_F64 }
	fn is_close_with_delta(&self, b: &Self, epsilon: &Self) -> bool {
		is_close_f64(*self, *b, *epsilon)
	}
}

/// Checks if two f32 values are within delta of each other.
pub fn is_close_f32(a: f32, b: f32, delta: f32) -> bool {
	abs_diff(a, b) < delta
}

/// Checks if two f64 values are within delta of each other.
pub fn is_close_f64(a: f64, b: f64, delta: f64) -> bool {
	abs_diff(a, b) < delta
}

/// Returns the absolute difference between two values.
pub fn abs_diff<T>(a: T, b: T) -> T
where
	T: PartialOrd + std::ops::Sub<Output = T>,
{
	if a > b { a - b } else { b - a }
}

impl CloseTo for Vec2 {
	fn default_delta() -> Self {
		Vec2::new(DEFAULT_DELTA_F32, DEFAULT_DELTA_F32)
	}
	fn is_close_with_delta(&self, b: &Self, delta: &Self) -> bool {
		is_close_f32(self.x, b.x, delta.x) && is_close_f32(self.y, b.y, delta.y)
	}
}



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
