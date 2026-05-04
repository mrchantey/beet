use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// Draw a single-line box border inside `rect` using box-drawing characters.
///
/// Uses per-side colors from [`VisualStyle`]: top/bottom colors for horizontal
/// segments and corners, left/right colors for vertical segments.
/// No-ops when the rect is too small to hold a border (width or height < 2).
pub(super) fn draw_border(
	buffer: &mut Buffer,
	rect: URect,
	node: &StyledNodeView,
) {
	let width = rect.width();
	let height = rect.height();

	if width < 2 || height < 2 {
		return; // too small for a border
	}

	let visual = node.visual_style();
	let entity = node.entity;

	// build per-side char styles
	let top_style = side_style(visual.border_top, visual);
	let bottom_style = side_style(visual.border_bottom, visual);
	let left_style = side_style(visual.border_left, visual);
	let right_style = side_style(visual.border_right, visual);

	// top border — corners use the top border color
	buffer.set(rect.min, Cell::new("┌", top_style.clone(), entity));
	for x in 1..width - 1 {
		buffer.set(
			UVec2::new(rect.min.x + x, rect.min.y),
			Cell::new("─", top_style.clone(), entity),
		);
	}
	buffer.set(
		UVec2::new(rect.min.x + width - 1, rect.min.y),
		Cell::new("┐", top_style.clone(), entity),
	);

	// middle rows — left and right sides only
	for y in 1..height - 1 {
		buffer.set(
			UVec2::new(rect.min.x, rect.min.y + y),
			Cell::new("│", left_style.clone(), entity),
		);
		buffer.set(
			UVec2::new(rect.min.x + width - 1, rect.min.y + y),
			Cell::new("│", right_style.clone(), entity),
		);
	}

	// bottom border — corners use the bottom border color
	buffer.set(
		UVec2::new(rect.min.x, rect.min.y + height - 1),
		Cell::new("└", bottom_style.clone(), entity),
	);
	for x in 1..width - 1 {
		buffer.set(
			UVec2::new(rect.min.x + x, rect.min.y + height - 1),
			Cell::new("─", bottom_style.clone(), entity),
		);
	}
	buffer.set(
		UVec2::new(rect.min.x + width - 1, rect.min.y + height - 1),
		Cell::new("┘", bottom_style.clone(), entity),
	);
}

/// Build a [`CharStyle`] for one border side, using the provided color as
/// the foreground and inheriting the background from the visual style.
fn side_style(border_color: Option<Color>, visual: &VisualStyle) -> CharStyle {
	CharStyle {
		foreground: border_color,
		background: visual.background,
		decoration_color: None,
		decoration_line: vec![],
	}
}
