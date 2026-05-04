use crate::prelude::*;
use crate::style::*;
use bevy::math::URect;
use bevy::math::UVec2;

pub fn border_layout(cx: &mut TuiRenderContext) {
	let width = cx.rect.width();
	let height = cx.rect.height();

	if width < 2 || height < 2 {
		return; // too small for border
	}
	let border = cx
		.node
		.layout
		.map(|layout| &layout.border)
		.unwrap_or(&Spacing::DEFAULT);
	if border == &Spacing::DEFAULT {
		return; // no border to draw
	}


	let style = cx.node.visual.unwrap_or(&VisualStyle::DEFAULT);

	let TuiRenderContext { buffer, rect, .. } = cx;

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
	cx.rect = URect::new(
		rect.min.x + 1,
		rect.min.y + 1,
		rect.max.x.saturating_sub(1),
		rect.max.y.saturating_sub(1),
	);
}
