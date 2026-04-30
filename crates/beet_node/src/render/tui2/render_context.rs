use crate::prelude::*;
use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::buffer::Buffer;
use ratatui::prelude::Rect;
// use ratatui::prelude::*;

#[derive(Get)]
pub struct TuiRenderContext<'a> {
	pub entity: EntityWorldMut<'a>,
	/// The full area of the terminal
	pub terminal_area: Rect,
	/// A subset of the terminal area, for the root
	/// this will be the same as the terminal area
	pub draw_area: Rect,
	pub buffer: &'a mut Buffer,
}



pub(super) fn render(world: &mut World) -> Result {
	world.resource_scope(
		|world: &mut World, mut context: Mut<RatatuiContext>| -> Result {
			let root_entity = world
				.query_filtered::<Entity, (With<EntityWidget>, Without<ChildOf>)>(
				)
				.single(world)?;

			// clone as we need &mut World
			let mut widget = world
				.entity(root_entity)
				.get::<EntityWidget>()
				.expect("just filtered")
				.clone();

			// capture the callback render result
			let mut result = None;
			context.draw(|frame| {
				result = widget
					.render(TuiRenderContext {
						entity: world.entity_mut(root_entity),
						// for top level, draw and terminal area are the same
						terminal_area: frame.area(),
						draw_area: frame.area(),
						buffer: frame.buffer_mut(),
					})
					.xsome();
			})?;
			// world.entity_mut(root_entity).insert(span_map);
			result.expect("certainly assigned")
		},
	)
}
