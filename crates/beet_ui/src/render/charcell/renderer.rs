use super::*;
use crate::style::StyledNodeQuery;
use beet_core::prelude::*;


pub fn render_charcell(
	styled_query: StyledNodeQuery,
	mut query: Query<(Entity, &mut CharcellRenderer)>,
) -> Result {
	for (entity, mut renderer) in query.iter_mut() {
		renderer.render_node(&styled_query, entity)?;
	}
	Ok(())
}

#[derive(Get, Deref, Component)]
pub struct CharcellRenderer {
	/// A buffer whose size matches the `viewport::size`
	#[deref]
	buffer: Buffer,
}

impl Default for CharcellRenderer {
	fn default() -> Self {
		let size = terminal_ext::size();
		Self::new_size(size.x, size.y)
	}
}

impl CharcellRenderer {
	pub fn new(viewport: URect) -> Self {
		Self {
			buffer: Buffer::new(viewport.size()),
		}
	}
	pub fn new_size(width: u32, height: u32) -> Self {
		Self::new(URect::new(0, 0, width, height))
	}

	/// Half the viewport height for an easier read when testing
	pub fn halved(mut self) -> Self {
		let mut size = self.size();
		size.y /= 2;
		self.buffer = Buffer::new(size);
		self
	}

	pub fn render_node(
		&mut self,
		query: &StyledNodeQuery,
		entity: Entity,
	) -> Result<&mut Self> {
		let node = query.get_view(entity);
		let size = self.size();
		let viewport = URect::new(0, 0, size.x, size.y);
		let mut cx = CharcellRenderContext::new(
			node,
			viewport,
			viewport,
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
}
