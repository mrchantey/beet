//! Box model measurement and rendering for the charcell layout engine.
//!
//! Provides [`BoxModel`] for computing margin/border/padding dimensions,
//! and three draw helpers that fill the corresponding terminal cells.
use crate::prelude::*;
use crate::style::BoxStyle;
use crate::style::Length;
use crate::style::Spacing;
use crate::style::VisualStyle;
use beet_core::prelude::*;
use bevy::math::IRect;
use bevy::math::IVec2;
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
	/// Explicit content width/height in cells (CSS `width`/`height`), `None` to
	/// size to content.
	pub width: Option<u32>,
	pub height: Option<u32>,
	/// Minimum content height in cells (CSS `min-height`), a floor the measure and
	/// layout passes grow the node up to. `None` for no floor.
	pub min_height: Option<u32>,
}

impl BoxModel {
	/// Compute the box model for `node` relative to `viewport`, with no containing
	/// block (the measure context, where a node's containing block is not yet
	/// known). Percentage `width`/`height` stay content-sized; absolute and
	/// viewport-relative lengths resolve.
	pub fn from_node(node: &CharcellNodeData, viewport: UVec2) -> Self {
		Self::from_box_style(node.box_style(), viewport, None)
	}

	/// Compute the box model for `node` with its `containing` block's content size
	/// known (the layout context), so a percentage `width`/`height` resolves
	/// against the containing block as CSS requires.
	pub fn from_node_in(
		node: &CharcellNodeData,
		viewport: UVec2,
		containing: UVec2,
	) -> Self {
		Self::from_box_style(node.box_style(), viewport, Some(containing))
	}

	/// Compute the box model from an optional [`BoxStyle`], the `viewport`, and an
	/// optional `containing` block content size (in cells).
	///
	/// Returns zeroed/false defaults when `box_style` is `None`. Explicit
	/// `width`/`height` resolve to cells: absolute lengths (1rem ≈ 1 cell, matching
	/// the gap/marker conversions) and viewport-relative lengths always resolve;
	/// a percentage resolves against `containing` when given (the layout pass) and
	/// otherwise stays content-sized (the measure pass, where the containing block
	/// is not yet known — matching how browsers treat percent against an auto
	/// container). So a `width: 100%` table still fills its column either way:
	/// through real percent resolution in layout, or block flow in measure.
	pub fn from_box_style(
		box_style: Option<&BoxStyle>,
		viewport: UVec2,
		containing: Option<UVec2>,
	) -> Self {
		let Some(box_style) = box_style else {
			return Self {
				margin: URect::default(),
				border: BorderSides::default(),
				padding: URect::default(),
				width: None,
				height: None,
				min_height: None,
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
			width: box_style.width.and_then(|length| {
				explicit_cells(length, vp, containing.map(|c| c.x))
			}),
			height: box_style.height.and_then(|length| {
				explicit_cells(length, vp, containing.map(|c| c.y))
			}),
			min_height: box_style.min_height.and_then(|length| {
				min_height_cells(length, viewport, containing.map(|c| c.y))
			}),
		}
	}

	/// The rect after subtracting margin from `containing`.
	pub fn border_rect(&self, containing: IRect) -> IRect {
		inset_rect(containing, self.margin)
	}

	/// The rect after subtracting margin and border from `containing`.
	///
	/// Shrinks by one cell per present border side.
	pub fn inner_rect(&self, containing: IRect) -> IRect {
		inset_rect(self.border_rect(containing), self.border.inset())
	}

	/// The rect after subtracting margin, border, and padding from `containing`.
	pub fn content_rect(&self, containing: IRect) -> IRect {
		inset_rect(self.inner_rect(containing), self.padding)
	}

	/// The bottom margin, in cells.
	pub fn margin_bottom(&self) -> u32 { self.margin.max.y }

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

/// The explicit border-box width/height a child reserves, resolved against its
/// `containing` block's content size (in cells), or `None` along an axis the
/// child sizes to content. Used by the layout pass to honour a percent `width`
/// (eg `width: 100%` filling a column, `width: 50%` taking half) that the measure
/// pass left content-sized because the containing block was not yet known.
///
/// CSS `width`/`height` are content-box here, so the child's own margin, border,
/// and padding overhead is added back to give the footprint that goes into its
/// rect — consistent with how [`measure_node`](super::measure_node) builds the
/// intrinsic size from an explicit content size.
pub(super) fn explicit_box_size(
	node: &CharcellNodeData,
	viewport: UVec2,
	containing: UVec2,
) -> (Option<u32>, Option<u32>) {
	let box_model = BoxModel::from_node_in(node, viewport, containing);
	let overhead = box_model.overhead();
	(
		box_model.width.map(|width| width + overhead.x),
		box_model.height.map(|height| height + overhead.y),
	)
}

/// The bottom margin a node reserves below itself, in cells. Block containers
/// drop this from their *last* child so a trailing margin doesn't pad the
/// container's bottom edge — the cross-target equivalent of the web
/// `:last-child { margin-bottom: 0 }` reset (blockquotes, cards, nested lists).
pub(super) fn node_bottom_margin(
	node: &CharcellNodeData,
	viewport: UVec2,
) -> u32 {
	BoxModel::from_node(node, viewport).margin_bottom()
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
	rect: IRect,
	sides: BorderSides,
	node: &CharcellNodeData,
	clip: Clip,
) {
	let box_style = node.box_style();
	let entity = node.entity;

	let width = rect.width();
	let height = rect.height();

	if width < 2 || height < 2 {
		return; // too small for a border
	}

	// Borders sit on the node's own background (CSS border-box), so an app bar's
	// bottom divider or a card's edge reads in that element's colour rather than
	// the page beneath. A node with no background of its own leaves this `None`,
	// composing the border over whatever fill already sits beneath it.
	let border_bg = node.visual_style().background;
	let top_style = side_style(box_style.and_then(|b| b.border_top), border_bg);
	let bottom_style =
		side_style(box_style.and_then(|b| b.border_bottom), border_bg);
	let left_style =
		side_style(box_style.and_then(|b| b.border_left), border_bg);
	let right_style =
		side_style(box_style.and_then(|b| b.border_right), border_bg);

	let (left, right) = (rect.min.x, rect.max.x - 1);
	let (top, bottom) = (rect.min.y, rect.max.y - 1);

	// per-side border weight: a thick rule (eg the blockquote callout's left
	// border) draws with the heavy box-drawing glyphs so the weight reads in the
	// terminal, where a border is otherwise a single light cell.
	let heavy = |width: fn(&BoxStyle) -> crate::style::Length| {
		box_style.map(|b| is_heavy(width(b))).unwrap_or(false)
	};
	let (heavy_top, heavy_bottom, heavy_left, heavy_right) = (
		heavy(|b| b.border.top),
		heavy(|b| b.border.bottom),
		heavy(|b| b.border.left),
		heavy(|b| b.border.right),
	);

	// a corner joins two sides; it reads heavy only when both meeting sides do.
	let corner = |a, b, heavy_glyph, light_glyph| {
		if a && b { heavy_glyph } else { light_glyph }
	};

	if sides.all() {
		// full box: corners join the sides
		buffer.set_composite_clipped(
			rect.min,
			Cell::new(
				corner(heavy_top, heavy_left, "┏", "┌"),
				top_style.clone(),
				entity,
			),
			clip,
		);
		buffer.set_composite_clipped(
			IVec2::new(right, top),
			Cell::new(
				corner(heavy_top, heavy_right, "┓", "┐"),
				top_style.clone(),
				entity,
			),
			clip,
		);
		buffer.set_composite_clipped(
			IVec2::new(left, bottom),
			Cell::new(
				corner(heavy_bottom, heavy_left, "┗", "└"),
				bottom_style.clone(),
				entity,
			),
			clip,
		);
		buffer.set_composite_clipped(
			IVec2::new(right, bottom),
			Cell::new(
				corner(heavy_bottom, heavy_right, "┛", "┘"),
				bottom_style.clone(),
				entity,
			),
			clip,
		);
	}

	// horizontal edges span the full width (corners overwrite the ends above)
	if sides.top {
		let glyph = if heavy_top { "━" } else { "─" };
		for x in left..=right {
			buffer.set_composite_clipped(
				IVec2::new(x, top),
				Cell::new(glyph, top_style.clone(), entity),
				clip,
			);
		}
	}
	if sides.bottom {
		let glyph = if heavy_bottom { "━" } else { "─" };
		for x in left..=right {
			buffer.set_composite_clipped(
				IVec2::new(x, bottom),
				Cell::new(glyph, bottom_style.clone(), entity),
				clip,
			);
		}
	}
	// vertical edges span the full height
	if sides.left {
		let glyph = if heavy_left { "┃" } else { "│" };
		for y in top..=bottom {
			buffer.set_composite_clipped(
				IVec2::new(left, y),
				Cell::new(glyph, left_style.clone(), entity),
				clip,
			);
		}
	}
	if sides.right {
		let glyph = if heavy_right { "┃" } else { "│" };
		for y in top..=bottom {
			buffer.set_composite_clipped(
				IVec2::new(right, y),
				Cell::new(glyph, right_style.clone(), entity),
				clip,
			);
		}
	}

	if sides.all() {
		// re-draw corners so they sit on top of the straight edges
		buffer.set_composite_clipped(
			rect.min,
			Cell::new(
				corner(heavy_top, heavy_left, "┏", "┌"),
				top_style.clone(),
				entity,
			),
			clip,
		);
		buffer.set_composite_clipped(
			IVec2::new(right, top),
			Cell::new(
				corner(heavy_top, heavy_right, "┓", "┐"),
				top_style,
				entity,
			),
			clip,
		);
		buffer.set_composite_clipped(
			IVec2::new(left, bottom),
			Cell::new(
				corner(heavy_bottom, heavy_left, "┗", "└"),
				bottom_style.clone(),
				entity,
			),
			clip,
		);
		buffer.set_composite_clipped(
			IVec2::new(right, bottom),
			Cell::new(
				corner(heavy_bottom, heavy_right, "┛", "┘"),
				bottom_style,
				entity,
			),
			clip,
		);
	}
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Resolve a `min-height` [`Length`] to whole cells, against the cell `viewport`
/// and an optional `containing` block height (in cells).
///
/// A viewport-relative minimum (`100vh`) is meaningless in an unbounded-height
/// buffer (the stdout [`FlexBuffer`](super::FlexBuffer), whose cell viewport
/// height is the [auto-grow sentinel](super::AUTO_GROW_VIEWPORT_HEIGHT)): it
/// would force a 65535-row fill. Such a floor resolves to `None` there, so the
/// node stays content-sized; absolute floors (eg `min-height: 10rem`) still
/// apply.
fn min_height_cells(
	length: Length,
	viewport: UVec2,
	containing: Option<u32>,
) -> Option<u32> {
	let viewport_relative = matches!(
		length,
		Length::ViewportHeight(_)
			| Length::ViewportMin(_)
			| Length::ViewportMax(_)
	);
	if viewport_relative && viewport.y >= super::AUTO_GROW_VIEWPORT_HEIGHT {
		return None;
	}
	let vp = Vec2::new(viewport.x as f32, viewport.y as f32);
	explicit_cells(length, vp, containing)
}

/// Resolve an explicit `width`/`height` [`Length`] to whole cells.
///
/// Absolute lengths use the rem convention (1rem ≈ 1 cell). Viewport-relative
/// lengths resolve as a fraction of the `viewport`, which here is measured in
/// *cells*, so `50vw` of a 20-cell-wide terminal is 10 cells (not the
/// pixel-denominated [`Length::into_rem`], which would shrink it ~16x). A
/// `Percent` resolves against the containing block's content size along the same
/// axis (`containing`, in cells) when known — the layout pass — returning `None`
/// otherwise so the measure pass sizes the node to content.
fn explicit_cells(
	length: Length,
	viewport: Vec2,
	containing: Option<u32>,
) -> Option<u32> {
	let cells =
		|fraction: f32, extent: f32| (fraction * extent).round().max(0.);
	match length {
		Length::Px(_) | Length::Rem(_) => {
			Some(length.into_rem(viewport).round().max(0.) as u32)
		}
		// percent is relative to the containing block, resolved in the layout pass
		Length::Percent(percent) => {
			containing.map(|axis| cells(percent / 100., axis as f32) as u32)
		}
		// viewport units resolve against the cell viewport, one cell per percent of
		// the relevant viewport extent
		Length::ViewportWidth(percent) => {
			Some(cells(percent / 100., viewport.x) as u32)
		}
		Length::ViewportHeight(percent) => {
			Some(cells(percent / 100., viewport.y) as u32)
		}
		Length::ViewportMin(percent) => {
			Some(cells(percent / 100., viewport.min_element()) as u32)
		}
		Length::ViewportMax(percent) => {
			Some(cells(percent / 100., viewport.max_element()) as u32)
		}
	}
}

/// A border at or above this width (in rem) draws with the heavy box-drawing
/// glyphs rather than the light ones. Sits between the thin (1px) and thick
/// (3px) outline tokens so only a deliberately thick rule reads as heavy.
const HEAVY_BORDER_REM: f32 = 0.15;

/// Whether a border of the given width draws heavy. Border widths are
/// pixel-based, so the viewport is irrelevant to the rem conversion.
fn is_heavy(width: crate::style::Length) -> bool {
	width.into_rem(Vec2::ZERO) >= HEAVY_BORDER_REM
}

/// Build a [`VisualStyle`] for one border side from its foreground color and an
/// optional background.
///
/// Border cells are written with [`set_composite`](AsBuffer::set_composite): a
/// `None` background composes over whatever fill sits beneath (used by lone
/// dividers), while `Some` paints the node's own surface under the border (used
/// by full boxes, so a bordered box's edge matches its fill).
pub(super) fn side_style(
	border_color: Option<Color>,
	background: Option<Color>,
) -> VisualStyle {
	VisualStyle {
		foreground: border_color,
		background,
		..default()
	}
}

/// Resolve a CSS inset (`top`/`right`/`bottom`/`left`) [`Length`] to signed
/// cells against the `containing` block size on the inset's axis.
///
/// Unlike width/height, an inset may be negative (shifting the box up/left), so
/// this keeps the sign rather than clamping to zero. Uses the same rem/viewport
/// conversion as margins/padding, including the x-axis doubling so a horizontal
/// inset reads consistently in the terminal's roughly 2:1 cell aspect. `x_axis`
/// is true for `left`/`right`.
pub(super) fn inset_cells(
	length: Length,
	viewport: UVec2,
	containing: u32,
	x_axis: bool,
) -> i32 {
	let vp = Vec2::new(viewport.x as f32, viewport.y as f32);
	let raw = match length {
		Length::Px(_) | Length::Rem(_) => length.into_rem(vp),
		Length::Percent(percent) => percent / 100. * containing as f32,
		Length::ViewportWidth(percent) => percent / 100. * vp.x,
		Length::ViewportHeight(percent) => percent / 100. * vp.y,
		Length::ViewportMin(percent) => percent / 100. * vp.min_element(),
		Length::ViewportMax(percent) => percent / 100. * vp.max_element(),
	};
	let cells = raw.round() as i32;
	if x_axis { cells * 2 } else { cells }
}

/// Translate a signed rect by `offset` (both corners), the scroll/paint shift.
pub(super) fn translate_rect(rect: IRect, offset: IVec2) -> IRect {
	IRect {
		min: rect.min + offset,
		max: rect.max + offset,
	}
}

/// Inset signed `outer` by the (non-negative) amounts in `insets`, returning the
/// shrunken rect. The `min` edges move inward by the leading insets and the
/// `max` edges by the trailing ones, in signed space so an off-screen or scrolled
/// box stays representable.
///
/// Returns `outer` unchanged when the result would be invalid (zero or
/// inverted dimensions).
pub(super) fn inset_rect(outer: IRect, insets: URect) -> IRect {
	let left = outer.min.x + insets.min.x as i32;
	let top = outer.min.y + insets.min.y as i32;
	let right = outer.max.x - insets.max.x as i32;
	let bottom = outer.max.y - insets.max.y as i32;

	if left >= right || top >= bottom {
		return outer;
	}

	IRect {
		min: IVec2::new(left, top),
		max: IVec2::new(right, bottom),
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
