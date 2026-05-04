use crate::prelude::*;
use crate::style::Spacing;
use crate::style::StyledNodeQuery;
use crate::style::StyledNodeView;
use beet_core::prelude::*;
use bevy::math::Vec2;

/// Rendering context passed through the node tree during a TUI render pass.
pub struct CharcellRenderContext<'a> {
	pub(super) node: StyledNodeView<'a>,
	/// Terminal viewport used for rem-unit calculations.
	pub(super) viewport: URect,
	/// The rect allocated by the parent (or the root rect for the root node).
	containing_block: URect,
	/// Content area after margin, border, and padding have been applied.
	pub(super) content_rect: URect,
	pub(super) buffer: &'a mut Buffer,
}

impl<'a> CharcellRenderContext<'a> {
	/// Construct a context for a child node inside `containing_block`.
	///
	/// Computes `content_rect` from the node's box model immediately.
	pub(super) fn new(
		node: StyledNodeView<'a>,
		viewport: URect,
		containing_block: URect,
		buffer: &'a mut Buffer,
	) -> Self {
		let box_model = BoxModel::from_node(&node, viewport);
		let content_rect = box_model.content_rect(containing_block);
		Self {
			node,
			viewport,
			containing_block,
			content_rect,
			buffer,
		}
	}

	/// Render into a half-height buffer sized to the current terminal.
	pub fn render_half(
		query: &StyledNodeQuery,
		entity: Entity,
	) -> Result<Buffer> {
		let mut size = terminal_size();
		size.y /= 2;
		Self::render_rect(query, entity, URect::new(0, 0, size.x, size.y))
	}

	/// Render into a full-size buffer sized to the current terminal.
	pub fn render_full(
		query: &StyledNodeQuery,
		entity: Entity,
	) -> Result<Buffer> {
		let size = terminal_size();
		Self::render_rect(query, entity, URect::new(0, 0, size.x, size.y))
	}

	/// Render into a buffer bounded by an explicit `rect`.
	pub fn render_rect(
		query: &StyledNodeQuery,
		entity: Entity,
		rect: URect,
	) -> Result<Buffer> {
		let mut buffer = Buffer::new(rect.size());
		let node = query.get_view(entity);
		let mut cx = CharcellRenderContext::new(node, rect, rect, &mut buffer);
		cx.render()?;
		buffer.xok()
	}

	/// Main entry point — draws border, then delegates to flex and text layout.
	pub fn render(&mut self) -> Result {
		let box_model = BoxModel::from_node(&self.node, self.viewport);

		// 1. draw border if the node has one
		if box_model.has_border {
			super::draw_border(
				self.buffer,
				box_model.border_rect(self.containing_block),
				&self.node,
			);
		}

		// 2. recompute content_rect (safe to call render() more than once)
		self.content_rect = box_model.content_rect(self.containing_block);

		// 3. flex layout
		super::flex_layout(self)?;

		// 4. text content
		super::text_layout(self)?;

		Ok(())
	}
}


/// Pure-data box model computed from a node's layout style.
///
/// Describes margin, border, and padding dimensions for a single node.
/// All values are in terminal cells (pixels).
pub(super) struct BoxModel {
	margin: URect,
	has_border: bool,
	padding: URect,
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


fn terminal_size() -> UVec2 {
	let default_size = UVec2::new(80, 24);
	cfg_if! {
		if #[cfg(feature = "crossterm")] {
			terminal_ext::size().unwrap_or(default_size)
		} else {
			default_size
		}
	}
}

/// Inset `outer` by the amounts in `insets`, returning the shrunken rect.
///
/// Returns `outer` unchanged when the result would be invalid (zero or
/// inverted dimensions).
fn inset_rect(outer: URect, insets: URect) -> URect {
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
fn tui_inset(spacing: &Spacing, viewport: Vec2) -> URect {
	let mut val = spacing.rem_urect(viewport);
	val.min.x *= 2;
	val.max.x *= 2;
	val
}
