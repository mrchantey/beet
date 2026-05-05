use super::*;
use crate::style::StyledNodeQuery;
use beet_core::prelude::*;


#[derive(Get, Deref, Component)]
pub struct CharcellRenderer {
	viewport: URect,
	/// A buffer whose size matches the `viewport::size`
	#[deref]
	buffer: Buffer,
}

impl Default for CharcellRenderer {
	fn default() -> Self {
		let size = Self::terminal_size();
		Self::new_size(size.x, size.y)
	}
}

impl CharcellRenderer {
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

	pub fn render_node(
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
			|entity, query| self.render_node(&query, entity),
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
