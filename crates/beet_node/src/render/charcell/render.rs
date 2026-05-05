use super::BoxModel;
use super::draw_border;
use super::draw_margin;
use super::draw_padding;
use crate::prelude::*;
use crate::style::StyledNodeQuery;
use crate::style::StyledNodeView;
use beet_core::prelude::*;

#[derive(Get, Deref)]
pub struct RenderCharcell {
	viewport: URect,
	#[deref]
	buffer: Buffer,
}

impl Default for RenderCharcell {
	fn default() -> Self {
		let size = Self::terminal_size();
		Self::new_size(size.x, size.y)
	}
}

impl RenderCharcell {
	pub fn new(viewport: URect) -> Self {
		Self {
			buffer: Buffer::new(viewport.size()),
			viewport,
		}
	}
	pub fn new_size(width: u32, height: u32) -> Self {
		Self::new(URect::new(0, 0, width, height))
	}

	/// Half the viewport height for an easier read when testing
	pub fn halved(mut self) -> Self {
		self.viewport.max.y /= 2;
		self
	}

	/// Render into a buffer bounded by an explicit `rect`.
	pub fn render_rect(
		&mut self,
		query: &StyledNodeQuery,
		entity: Entity,
	) -> Result<&mut Self> {
		let node = query.get_view(entity);
		let mut cx = CharcellRenderContext::new(
			node,
			self.viewport,
			self.viewport,
			&mut self.buffer,
		);
		cx.render()?;
		self.xok()
	}

	/// Create a world, spawn the bundle and render to a buffer
	pub fn render_oneshot(&mut self, bundle: impl Bundle) -> Result<&mut Self> {
		World::new().spawn(bundle).with_state::<StyledNodeQuery, _>(
			|entity, query| self.render_rect(&query, entity),
		)
	}

	fn terminal_size() -> UVec2 {
		// standard default terminal size
		let default_size = UVec2::new(80, 24);
		cfg_if! {
			if #[cfg(feature = "crossterm")] {
				terminal_ext::size().unwrap_or(default_size)
			} else {
				default_size
			}
		}
	}
}


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
	parent: Option<(Entity, CharStyle)>,
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
		style: impl Into<CharStyle>,
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
				self.node.visual_style().clone().into(),
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
