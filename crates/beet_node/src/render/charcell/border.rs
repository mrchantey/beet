use crate::prelude::*;
use crate::style::*;
use bevy::math::URect;
use bevy::math::UVec2;

/// Draw a single-line box border inside `rect` using box-drawing characters.
///
/// No-ops when the rect is too small to hold a border (width or height < 2).
pub(super) fn draw_border(
	buffer: &mut Buffer,
	rect: URect,
	style: &VisualStyle,
) {
	let width = rect.width();
	let height = rect.height();

	if width < 2 || height < 2 {
		return; // too small for a border
	}

	// top border
	buffer.set(rect.min, Cell::new("┌", style.clone()));
	for x in 1..width - 1 {
		buffer.set(
			UVec2::new(rect.min.x + x, rect.min.y),
			Cell::new("─", style.clone()),
		);
	}
	buffer.set(
		UVec2::new(rect.min.x + width - 1, rect.min.y),
		Cell::new("┐", style.clone()),
	);

	// middle rows — left and right sides only
	for y in 1..height - 1 {
		buffer.set(
			UVec2::new(rect.min.x, rect.min.y + y),
			Cell::new("│", style.clone()),
		);
		buffer.set(
			UVec2::new(rect.min.x + width - 1, rect.min.y + y),
			Cell::new("│", style.clone()),
		);
	}

	// bottom border
	buffer.set(
		UVec2::new(rect.min.x, rect.min.y + height - 1),
		Cell::new("└", style.clone()),
	);
	for x in 1..width - 1 {
		buffer.set(
			UVec2::new(rect.min.x + x, rect.min.y + height - 1),
			Cell::new("─", style.clone()),
		);
	}
	buffer.set(
		UVec2::new(rect.min.x + width - 1, rect.min.y + height - 1),
		Cell::new("┘", style.clone()),
	);
}
