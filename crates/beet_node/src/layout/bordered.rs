use super::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// A widget that wraps a child in a border.
pub struct Bordered {
	pub child: Box<dyn 'static + Send + Sync + Widget>,
	pub layout_style: LayoutStyle,
}

impl Bordered {
	pub fn new(child: impl 'static + Send + Sync + Widget) -> Self {
		Self {
			child: Box::new(child),
			layout_style: LayoutStyle::default(),
		}
	}

	pub fn flex_grow(mut self, grow: u32) -> Self {
		self.layout_style.flex_grow = grow;
		self
	}
}

impl Widget for Bordered {
	fn layout_style(&self) -> &LayoutStyle { &self.layout_style }

	fn measure(&self, available: UVec2) -> UVec2 {
		// border takes 2 units on each axis
		let inner_available = UVec2::new(
			available.x.saturating_sub(2),
			available.y.saturating_sub(2),
		);
		let child_size = self.child.measure(inner_available);
		UVec2::new(
			child_size.x.saturating_add(2),
			child_size.y.saturating_add(2),
		)
	}

	fn layout(&self, buffer: &mut Buffer, rect: URect) {
		let width = rect.width();
		let height = rect.height();

		if width < 2 || height < 2 {
			return; // too small for border
		}

		let style = VisualStyle::default();

		// top border
		buffer.set(rect.min, Cell::new("┌").with_style(style.clone()));
		for x in 1..width - 1 {
			buffer.set(
				UVec2::new(rect.min.x + x, rect.min.y),
				Cell::new("─").with_style(style.clone()),
			);
		}
		buffer.set(
			UVec2::new(rect.min.x + width - 1, rect.min.y),
			Cell::new("┐").with_style(style.clone()),
		);

		// middle rows with sides
		for y in 1..height - 1 {
			buffer.set(
				UVec2::new(rect.min.x, rect.min.y + y),
				Cell::new("│").with_style(style.clone()),
			);
			buffer.set(
				UVec2::new(rect.min.x + width - 1, rect.min.y + y),
				Cell::new("│").with_style(style.clone()),
			);
		}

		// bottom border
		buffer.set(
			UVec2::new(rect.min.x, rect.min.y + height - 1),
			Cell::new("└").with_style(style.clone()),
		);
		for x in 1..width - 1 {
			buffer.set(
				UVec2::new(rect.min.x + x, rect.min.y + height - 1),
				Cell::new("─").with_style(style.clone()),
			);
		}
		buffer.set(
			UVec2::new(rect.min.x + width - 1, rect.min.y + height - 1),
			Cell::new("┘").with_style(style.clone()),
		);

		// layout child in inner rect
		let inner_rect = URect::new(
			rect.min.x + 1,
			rect.min.y + 1,
			rect.max.x.saturating_sub(1),
			rect.max.y.saturating_sub(1),
		);
		self.child.layout(buffer, inner_rect);
	}
}
