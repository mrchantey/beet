use super::*;
use crate::style::StyledNodeView;
use crate::style::VisualStyle;
use beet_core::prelude::*;


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
	/// The parent entity and visual style, used to fill margin cells.
	parent: Option<(Entity, VisualStyle)>,
}

impl<'a> CharcellRenderContext<'a> {
	/// Construct a context for a node inside `containing_block`.
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
			parent: None,
		}
	}

	/// Set the parent entity and style, used to fill margin cells.
	pub(super) fn with_parent(
		mut self,
		entity: Entity,
		style: impl Into<VisualStyle>,
	) -> Self {
		self.parent = Some((entity, style.into()));
		self
	}

	/// Main entry point — draws margin, border, padding, then delegates to
	/// flex and text layout.
	pub fn render(&mut self) -> Result {
		let box_model = BoxModel::from_node(&self.node, self.viewport);

		// 1. fill margin cells with the parent entity/style
		let border_rect = box_model.border_rect(self.containing_block);
		if let Some((parent_entity, ref parent_style)) = self.parent {
			if border_rect != self.containing_block {
				draw_margin(
					self.buffer,
					self.containing_block,
					border_rect,
					parent_style.clone(),
					parent_entity,
				);
			}
		}

		// 2. draw border if the node has one
		if box_model.has_border {
			draw_border(self.buffer, border_rect, &self.node);
		}

		// 3. fill padding cells with the current node entity/style
		let inner_rect = box_model.inner_rect(self.containing_block);
		let content_rect = box_model.content_rect(self.containing_block);
		if content_rect != inner_rect {
			draw_padding(
				self.buffer,
				inner_rect,
				content_rect,
				self.node.visual_style().clone(),
				self.node.entity,
			);
		}

		// 4. recompute content_rect (safe to call render() more than once)
		self.content_rect = content_rect;

		// 5. flex layout
		super::flex_layout(self)?;

		// 6. text content
		super::text_layout(self)?;

		Ok(())
	}
}
