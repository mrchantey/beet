pub const DEFAULT_DELTA_F32: f32 = 0.1;
pub const DEFAULT_DELTA_F64: f64 = 0.1;

pub trait CloseTo: Sized {
	fn default_delta() -> Self;
	fn is_close_with_delta(&self, b: &Self, epsilon: &Self) -> bool;
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

pub fn is_close_f32(a: f32, b: f32, delta: f32) -> bool {
	abs_diff(a, b) < delta
}

pub fn is_close_f64(a: f64, b: f64, delta: f64) -> bool {
	abs_diff(a, b) < delta
}

pub fn abs_diff<T>(a: T, b: T) -> T
where
	T: PartialOrd + std::ops::Sub<Output = T>,
{
	if a > b { a - b } else { b - a }
}
