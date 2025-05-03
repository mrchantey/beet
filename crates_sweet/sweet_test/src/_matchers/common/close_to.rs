pub const DEFAULT_EPSILON_F32: f32 = 0.1;
pub const DEFAULT_EPSILON_F64: f64 = 0.1;

pub trait CloseTo: Sized {
    fn default_epsilon() -> Self;
    fn is_close_with_epsilon(a: Self, b: Self, epsilon: Self) -> bool;
    fn is_close(a: Self, b: Self) -> bool {
        Self::is_close_with_epsilon(a, b, Self::default_epsilon())
    }

    //  {
    // 	abs_diff(a, b) < epsilon
    // }
}
impl CloseTo for f32 {
    fn default_epsilon() -> Self {
        DEFAULT_EPSILON_F32
    }
    fn is_close_with_epsilon(a: Self, b: Self, epsilon: Self) -> bool {
        is_close_f32(a, b, epsilon)
    }
}
impl CloseTo for f64 {
    fn default_epsilon() -> Self {
        DEFAULT_EPSILON_F64
    }
    fn is_close_with_epsilon(a: Self, b: Self, epsilon: Self) -> bool {
        is_close_f64(a, b, epsilon)
    }
}

pub fn is_close_f32(a: f32, b: f32, epsilon: f32) -> bool {
    abs_diff(a, b) < epsilon
}

pub fn is_close_f64(a: f64, b: f64, epsilon: f64) -> bool {
    abs_diff(a, b) < epsilon
}

pub fn abs_diff<T>(a: T, b: T) -> T
where
    T: PartialOrd + std::ops::Sub<Output = T>,
{
    if a > b {
        a - b
    } else {
        b - a
    }
}
