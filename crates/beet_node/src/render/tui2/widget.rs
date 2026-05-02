use crate::prelude::*;
use beet_core::prelude::*;

// ── Widget trait ──────────────────────────────────────────────────────────────

pub trait Widget {
	fn layout_style(&self) -> &LayoutStyle;

	/// Pass 1 (bottom-up): given available space as a hint, return desired size.
	fn measure(&self, available: UVec2) -> UVec2;

	/// Pass 2 (top-down): given the assigned rect, emit render cells to buffer.
	fn layout(&self, buffer: &mut Buffer, rect: URect);
}

#[derive(Component, Deref, DerefMut)]
pub struct EntityWidget {
	widget: Box<dyn 'static + Send + Sync + Widget>,
}

impl EntityWidget {
	pub fn new(render: impl 'static + Send + Sync + Widget) -> Self {
		Self {
			widget: Box::new(render),
		}
	}
}
