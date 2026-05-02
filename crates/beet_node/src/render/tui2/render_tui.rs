use crate::prelude::*;
use crate::style::Spacing;
use beet_core::prelude::*;

pub struct TuiRenderContext<'a> {
	pub node: &'a StyledNodeView<'a>,
	pub viewport: URect,
	pub rect: URect,
	pub buffer: &'a mut Buffer,
}

impl<'a> TuiRenderContext<'a> {
	pub fn render_half(
		query: &StyledNodeQuery,
		entity: Entity,
	) -> Result<Buffer> {
		let mut size =
			terminal_ext::size().unwrap_or_else(|_| UVec2::new(80, 24));
		size.y /= 2;
		Self::render_rect(query, entity, URect::new(0, 0, size.x, size.y))
	}
	pub fn render(query: &StyledNodeQuery, entity: Entity) -> Result<Buffer> {
		let viewport =
			terminal_ext::size().unwrap_or_else(|_| UVec2::new(80, 24));
		Self::render_rect(
			query,
			entity,
			URect::new(0, 0, viewport.x, viewport.y),
		)
	}
	pub fn render_rect(
		query: &StyledNodeQuery,
		entity: Entity,
		rect: URect,
	) -> Result<Buffer> {
		let mut buffer = Buffer::new(rect);
		let node = query.get_view(entity);
		let mut this = TuiRenderContext {
			node: &node,
			viewport: rect,
			rect,
			buffer: &mut buffer,
		};
		layout(&mut this)?;
		buffer.xok()
	}
	pub fn viewport_size(&self) -> Vec2 {
		Vec2::new(self.viewport.width() as f32, self.viewport.height() as f32)
	}

	/// Double the spacing on the x axis, for rem consistency
	fn tui_spacing(&self, spacing: &Spacing) -> URect {
		let viewport = self.viewport_size();
		let mut val = spacing.rem_urect(viewport);
		val.min.x *= 2;
		val.max.x *= 2;
		val
	}
}

fn layout(cx: &mut TuiRenderContext) -> Result {
	content_box_layout(cx)?;
	text_layout(cx)?;
	Ok(())
}

fn content_box_layout(cx: &mut TuiRenderContext) -> Result {
	let Some(layout) = cx.node.layout else {
		return Ok(());
	};

	// 1. margin
	let margin = cx.tui_spacing(&layout.margin);
	let border_rect = subtract_rect(&cx.rect, &margin);
	cx.rect = border_rect;

	// 2. border
	border_layout(cx);

	// 3. padding
	let padding = cx.tui_spacing(&layout.padding);
	let content_rect = subtract_rect(&cx.rect, &padding);
	cx.rect = content_rect;

	Ok(())
}

/// For a given outer rect, create a new rect by moving
/// inward at all points
fn subtract_rect(outer: &URect, inner: &URect) -> URect {
	let left = outer.min.x + inner.min.x;
	let top = outer.min.y + inner.min.y;
	let right = outer.max.x.saturating_sub(inner.max.x);
	let bottom = outer.max.y.saturating_sub(inner.max.y);
	URect {
		min: UVec2::new(left, top),
		max: UVec2::new(right, bottom),
	}
}
