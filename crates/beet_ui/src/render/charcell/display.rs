use super::*;
use crate::style::Display;
use beet_core::prelude::*;

/// Select and run the layout algorithm for a node's children based on [`Display`].
pub fn display_layout(cx: &mut CharcellRenderContext) -> Result {
	match cx.node.layout_style().display {
		Display::Flex => super::flex_layout(cx),
		Display::Block => block_layout(cx),
		Display::Inline => inline_layout(cx),
	}
}

/// Block flow: render children top-to-bottom, each taking full width.
pub fn block_layout(cx: &mut CharcellRenderContext) -> Result {
	if cx.node.children.is_empty() {
		return Ok(());
	}
	let children = cx.node.children.clone();
	let parent_entity = cx.node.entity;
	let parent_style = cx.node.visual_style().clone();
	let content_rect = cx.content_rect;
	let viewport = cx.viewport;
	let mut child_y = content_rect.min.y;
	for child in children {
		if child_y >= content_rect.max.y {
			break;
		}
		let available_h = content_rect
			.height()
			.saturating_sub(child_y - content_rect.min.y);
		let child_size = super::text_measure(
			&child,
			UVec2::new(content_rect.width(), available_h),
		)?;
		let child_rect = URect::new(
			content_rect.min.x,
			child_y,
			content_rect.max.x,
			content_rect.max.y,
		);
		CharcellRenderContext::new(child, viewport, child_rect, cx.buffer)
			.with_parent(parent_entity, parent_style.clone())
			.render()?;
		child_y += child_size.y.max(1);
	}
	Ok(())
}

/// Inline flow: for now uses the same algorithm as block layout.
///
/// Full inline (side-by-side) rendering is not yet implemented.
pub fn inline_layout(cx: &mut CharcellRenderContext) -> Result {
	block_layout(cx)
}
