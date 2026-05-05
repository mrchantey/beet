//! Box model measurement and rendering for the charcell layout engine.
//!
//! Provides [`BoxModel`] for computing margin/border/padding dimensions,
//! and three draw helpers that fill the corresponding terminal cells.
use crate::prelude::*;
use crate::render::Buffer;
use crate::style::Spacing;
use crate::style::StyledNodeView;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;
use bevy::math::Vec2;

// ── BoxModel ─────────────────────────────────────────────────────────────────

/// Pure-data box model computed from a node's layout style.
///
/// Describes margin, border, and padding dimensions for a single node.
/// All values are in terminal cells.
pub(super) struct BoxModel {
	pub margin: URect,
	pub has_border: bool,
	pub padding: URect,
}

impl BoxModel {
	/// Compute the box model for `node` relative to `viewport`.
	///
	/// Returns zeroed/false defaults when the node has no layout style.
	pub fn from_node(node: &StyledNodeView, viewport: URect) -> Self {
		let Some(layout) = node.layout else {
			return Self {
				margin: URect::default(),
				has_border: false,
				padding: URect::default(),
			};
		};

		let vp = Vec2::new(viewport.width() as f32, viewport.height() as f32);
		let margin = tui_inset(&layout.margin, vp);
		let padding = tui_inset(&layout.padding, vp);
		let has_border = layout.border != Spacing::DEFAULT;

		Self {
			margin,
			has_border,
			padding,
		}
	}

	/// The rect after subtracting margin from `containing`.
	pub fn border_rect(&self, containing: URect) -> URect {
		inset_rect(containing, self.margin)
	}

	/// The rect after subtracting margin and border from `containing`.
	///
	/// Shrinks by 1 cell on every side when `has_border` is true.
	pub fn inner_rect(&self, containing: URect) -> URect {
		let border = self.border_rect(containing);
		if self.has_border {
			inset_rect(border, URect {
				min: UVec2::new(1, 1),
				max: UVec2::new(1, 1),
			})
		} else {
			border
		}
	}

	/// The rect after subtracting margin, border, and padding from `containing`.
	pub fn content_rect(&self, containing: URect) -> URect {
		inset_rect(self.inner_rect(containing), self.padding)
	}

	/// Total cell overhead consumed by margin, border, and padding.
	pub fn overhead(&self) -> UVec2 {
		let margin_x = self.margin.min.x + self.margin.max.x;
		let margin_y = self.margin.min.y + self.margin.max.y;
		let padding_x = self.padding.min.x + self.padding.max.x;
		let padding_y = self.padding.min.y + self.padding.max.y;
		let border_x = if self.has_border { 2 } else { 0 };
		let border_y = if self.has_border { 2 } else { 0 };
		UVec2::new(
			margin_x + padding_x + border_x,
			margin_y + padding_y + border_y,
		)
	}
}

// ── Drawing ───────────────────────────────────────────────────────────────────

/// Fill margin cells — the area between `containing` and `border_rect` —
/// with the parent entity and style. No-op when there is no margin.
pub(super) fn draw_margin(
	buffer: &mut Buffer,
	containing: URect,
	border_rect: URect,
	style: CharStyle,
	entity: Entity,
) {
	fill_frame(buffer, containing, border_rect, style, entity);
}

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

/// Fill padding cells — the area between `inner_rect` and `content_rect` —
/// with the current node entity and style. No-op when there is no padding.
pub(super) fn draw_padding(
	buffer: &mut Buffer,
	inner_rect: URect,
	content_rect: URect,
	style: CharStyle,
	entity: Entity,
) {
	fill_frame(buffer, inner_rect, content_rect, style, entity);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Fill every cell in `outer` that lies outside `inner` with a space cell.
fn fill_frame(
	buffer: &mut Buffer,
	outer: URect,
	inner: URect,
	style: CharStyle,
	entity: Entity,
) {
	// clamp inner to outer so we never write outside it
	let inner_min_x = inner.min.x.max(outer.min.x);
	let inner_min_y = inner.min.y.max(outer.min.y);
	let inner_max_x = inner.max.x.min(outer.max.x);
	let inner_max_y = inner.max.y.min(outer.max.y);

	for y in outer.min.y..outer.max.y {
		for x in outer.min.x..outer.max.x {
			let inside = x >= inner_min_x
				&& x < inner_max_x
				&& y >= inner_min_y
				&& y < inner_max_y;
			if !inside {
				buffer.set(
					UVec2::new(x, y),
					Cell::new(" ", style.clone(), entity),
				);
			}
		}
	}
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

/// Inset `outer` by the amounts in `insets`, returning the shrunken rect.
///
/// Returns `outer` unchanged when the result would be invalid (zero or
/// inverted dimensions).
pub(super) fn inset_rect(outer: URect, insets: URect) -> URect {
	let left = outer.min.x + insets.min.x;
	let top = outer.min.y + insets.min.y;
	let right = outer.max.x.saturating_sub(insets.max.x);
	let bottom = outer.max.y.saturating_sub(insets.max.y);

	if left >= right || top >= bottom {
		return outer;
	}

	URect {
		min: UVec2::new(left, top),
		max: UVec2::new(right, bottom),
	}
}

/// Convert a `Spacing` value to a `URect` in terminal cells.
///
/// Doubles the x-axis values so rem units are visually consistent with
/// terminal fonts (which are roughly twice as tall as they are wide).
pub(super) fn tui_inset(spacing: &Spacing, viewport: Vec2) -> URect {
	let mut val = spacing.rem_urect(viewport);
	val.min.x *= 2;
	val.max.x *= 2;
	val
}
