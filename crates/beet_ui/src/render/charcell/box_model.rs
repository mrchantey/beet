//! Box model measurement and rendering for the charcell layout engine.
//!
//! Provides [`BoxModel`] for computing margin/border/padding dimensions,
//! and three draw helpers that fill the corresponding terminal cells.
use crate::prelude::*;
use crate::render::Buffer;
use crate::style::BoxStyle;
use crate::style::Spacing;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::URect;
use bevy::math::UVec2;
use bevy::math::Vec2;

use super::query::CharcellNodeData;

// ── BoxModel ─────────────────────────────────────────────────────────────────

/// Pure-data box model computed from a node's box style.
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
				has_border: false,
				padding: URect::default(),
			};
		};

		let vp = Vec2::new(viewport.x as f32, viewport.y as f32);
		let margin = tui_inset(&box_style.margin, vp);
		let padding = tui_inset(&box_style.padding, vp);
		let has_border = box_style.border != Spacing::DEFAULT;

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

// ── Drawing ─────────────────────────────────────────────────────────────────

/// Draw a single-line box border inside `rect` using box-drawing characters.
///
/// Uses per-side colors from [`BoxStyle`]: top/bottom colors for horizontal
/// segments and corners, left/right colors for vertical segments.
/// No-ops when the rect is too small to hold a border (width or height < 2).
pub(super) fn draw_border(
	buffer: &mut Buffer,
	rect: URect,
	box_style: Option<&BoxStyle>,
	visual: &VisualStyle,
	entity: Entity,
) {
	let width = rect.width();
	let height = rect.height();

	if width < 2 || height < 2 {
		return; // too small for a border
	}

	// build per-side char styles
	let top_style = side_style(box_style.and_then(|b| b.border_top), visual);
	let bottom_style =
		side_style(box_style.and_then(|b| b.border_bottom), visual);
	let left_style = side_style(box_style.and_then(|b| b.border_left), visual);
	let right_style =
		side_style(box_style.and_then(|b| b.border_right), visual);

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
