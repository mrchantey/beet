//! Box model measurement and rendering for the charcell layout engine.
//!
//! Provides [`BoxModel`] for computing margin/border/padding dimensions,
//! and three draw helpers that fill the corresponding terminal cells.
use crate::prelude::*;
use crate::style::BoxStyle;
use crate::style::Spacing;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;
use bevy::math::Vec2;

use super::query::CharcellNodeData;

// ── BoxModel ─────────────────────────────────────────────────────────────────

/// Which sides of a box carry a border. Each present side reserves and paints a
/// single terminal cell, so a node can have just a right border (a sidebar
/// divider) or just a bottom border (an elevated bar) rather than a full box.
#[derive(Default, Clone, Copy)]
pub(super) struct BorderSides {
	pub left: bool,
	pub right: bool,
	pub top: bool,
	pub bottom: bool,
}

impl BorderSides {
	/// Whether every side carries a border (a full box, drawn with corners).
	pub fn all(&self) -> bool {
		self.left && self.right && self.top && self.bottom
	}
	/// Whether any side carries a border.
	pub fn any(&self) -> bool {
		self.left || self.right || self.top || self.bottom
	}
	/// Margin-style inset (in cells) reserved by the present sides.
	fn inset(&self) -> URect {
		URect {
			min: UVec2::new(self.left as u32, self.top as u32),
			max: UVec2::new(self.right as u32, self.bottom as u32),
		}
	}
}

/// Pure-data box model computed from a node's box style.
///
/// Describes margin, border, and padding dimensions for a single node.
/// All values are in terminal cells.
pub(super) struct BoxModel {
	pub margin: URect,
	pub border: BorderSides,
	pub padding: URect,
}

impl BoxModel {
	/// Compute the box model for `node` relative to `viewport`.
	///
	/// Returns zeroed/false defaults when the node has no box style.
	pub fn from_node(node: &CharcellNodeData, viewport: UVec2) -> Self {
		Self::from_box_style(node.box_style(), viewport)
	}

	/// Compute the box model from an optional [`BoxStyle`] and `viewport`.
	///
	/// Returns zeroed/false defaults when `box_style` is `None`.
	pub fn from_box_style(
		box_style: Option<&BoxStyle>,
		viewport: UVec2,
	) -> Self {
		let Some(box_style) = box_style else {
			return Self {
				margin: URect::default(),
				border: BorderSides::default(),
				padding: URect::default(),
			};
		};

		let vp = Vec2::new(viewport.x as f32, viewport.y as f32);
		let margin = tui_inset(&box_style.margin, vp);
		let padding = tui_inset(&box_style.padding, vp);
		// a side carries a border when it has a positive width; per-side widths
		// let a rule reserve a single edge (eg `border-right`).
		let border = BorderSides {
			left: box_style.border.left.into_rem(vp) > 0.,
			right: box_style.border.right.into_rem(vp) > 0.,
			top: box_style.border.top.into_rem(vp) > 0.,
			bottom: box_style.border.bottom.into_rem(vp) > 0.,
		};

		Self {
			margin,
			border,
			padding,
		}
	}

	/// The rect after subtracting margin from `containing`.
	pub fn border_rect(&self, containing: URect) -> URect {
		inset_rect(containing, self.margin)
	}

	/// The rect after subtracting margin and border from `containing`.
	///
	/// Shrinks by one cell per present border side.
	pub fn inner_rect(&self, containing: URect) -> URect {
		inset_rect(self.border_rect(containing), self.border.inset())
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
		let border_x = self.border.left as u32 + self.border.right as u32;
		let border_y = self.border.top as u32 + self.border.bottom as u32;
		UVec2::new(
			margin_x + padding_x + border_x,
			margin_y + padding_y + border_y,
		)
	}
}

// ── Drawing ─────────────────────────────────────────────────────────────────

/// Draw the present border sides inside `rect` using box-drawing characters.
///
/// A full box (all four sides) is drawn with corners; otherwise each present
/// side is drawn as a straight edge, so a lone right border becomes a vertical
/// divider and a lone bottom border an underline. Per-side colors come from
/// [`BoxStyle`]. No-ops when the rect is too small (width or height < 2).
pub(super) fn draw_border(
	buffer: &mut impl AsBuffer,
	rect: URect,
	sides: BorderSides,
	node: &CharcellNodeData,
) {
	let box_style = node.box_style();
	let visual = node.visual_style();
	let entity = node.entity;

	let width = rect.width();
	let height = rect.height();

	if width < 2 || height < 2 {
		return; // too small for a border
	}

	let top_style = side_style(box_style.and_then(|b| b.border_top), visual);
	let bottom_style =
		side_style(box_style.and_then(|b| b.border_bottom), visual);
	let left_style = side_style(box_style.and_then(|b| b.border_left), visual);
	let right_style =
		side_style(box_style.and_then(|b| b.border_right), visual);

	let (left, right) = (rect.min.x, rect.max.x - 1);
	let (top, bottom) = (rect.min.y, rect.max.y - 1);

	if sides.all() {
		// full box: corners join the sides
		buffer.set(rect.min, Cell::new("┌", top_style.clone(), entity));
		buffer.set(UVec2::new(right, top), Cell::new("┐", top_style.clone(), entity));
		buffer.set(UVec2::new(left, bottom), Cell::new("└", bottom_style.clone(), entity));
		buffer.set(UVec2::new(right, bottom), Cell::new("┘", bottom_style.clone(), entity));
	}

	// horizontal edges span the full width (corners overwrite the ends above)
	if sides.top {
		for x in left..=right {
			buffer.set(UVec2::new(x, top), Cell::new("─", top_style.clone(), entity));
		}
	}
	if sides.bottom {
		for x in left..=right {
			buffer.set(UVec2::new(x, bottom), Cell::new("─", bottom_style.clone(), entity));
		}
	}
	// vertical edges span the full height
	if sides.left {
		for y in top..=bottom {
			buffer.set(UVec2::new(left, y), Cell::new("│", left_style.clone(), entity));
		}
	}
	if sides.right {
		for y in top..=bottom {
			buffer.set(UVec2::new(right, y), Cell::new("│", right_style.clone(), entity));
		}
	}

	if sides.all() {
		// re-draw corners so they sit on top of the straight edges
		buffer.set(rect.min, Cell::new("┌", top_style.clone(), entity));
		buffer.set(UVec2::new(right, top), Cell::new("┐", top_style, entity));
		buffer.set(UVec2::new(left, bottom), Cell::new("└", bottom_style.clone(), entity));
		buffer.set(UVec2::new(right, bottom), Cell::new("┘", bottom_style, entity));
	}
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a [`VisualStyle`] for one border side, using the provided color as
/// the foreground and inheriting the background from the visual style.
pub(super) fn side_style(
	border_color: Option<Color>,
	visual: &VisualStyle,
) -> VisualStyle {
	VisualStyle {
		foreground: border_color,
		background: visual.background,
		..default()
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
