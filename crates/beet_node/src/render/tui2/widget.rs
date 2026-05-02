use crate::prelude::*;
use crate::style::LayoutStyle;
use beet_core::prelude::*;

pub trait Widget {
	fn layout_style(&self) -> &LayoutStyle;

	/// Pass 1 (bottom-up): given available space as a hint, return desired size.
	fn measure(&self, available: UVec2) -> UVec2;

	/// Pass 2 (top-down): given the assigned rect, emit render cells to buffer.
	fn layout(&self, buffer: &mut Buffer, rect: URect);
}
