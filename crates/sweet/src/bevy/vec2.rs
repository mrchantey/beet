use crate::prelude::*;
use bevy::prelude::*;

impl CloseTo for Vec2 {
	fn default_delta() -> Self {
		Vec2::new(DEFAULT_DELTA_F32, DEFAULT_DELTA_F32)
	}
	fn is_close_with_delta(&self, b: &Self, delta: &Self) -> bool {
		is_close_f32(self.x, b.x, delta.x) && is_close_f32(self.y, b.y, delta.y)
	}
}
