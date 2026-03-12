use std::sync::Arc;

use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::prelude::Rect;
use ratatui::prelude::*;


#[derive(Clone, Component)]
pub struct TuiWidget {
	render: Arc<
		dyn 'static
			+ Send
			+ Sync
			+ Fn(EntityWorldMut, Rect, &mut Buffer) -> Result,
	>,
}


impl TuiWidget {
	pub fn new(
		render: impl 'static
		+ Send
		+ Sync
		+ Fn(EntityWorldMut, Rect, &mut Buffer) -> Result,
	) -> Self {
		Self {
			render: Arc::new(render),
		}
	}

	pub fn render(
		&mut self,
		world: EntityWorldMut,
		area: Rect,
		buf: &mut Buffer,
	) -> Result {
		(self.render)(world, area, buf)
	}
}

/// Render the widget tree to the terminal. This runs on
/// any `Changed<TuiWidget>`, and renders as a hierarchy
/// starting from the root widget: [`TuiWidget,Without<ChildOf>`]
pub fn render_widgets(world: &mut World) -> Result {
	world.resource_scope(
		|world: &mut World, mut context: Mut<RatatuiContext>| -> Result {
			let root_entity = world
				.query_filtered::<Entity, (With<TuiWidget>, Without<ChildOf>)>()
				.single(world)?;

			// clone as we need &mut World
			let mut widget = world
				.entity(root_entity)
				.get::<TuiWidget>()
				.expect("just filtered")
				.clone();

			// capture the callback render result
			let mut result = None;
			context.draw(|frame| {
				let area = frame.area();
				let buf = frame.buffer_mut();
				result = widget
					.render(world.entity_mut(root_entity), area, buf)
					.xsome();
			})?;
			result.expect("certainly assigned")
		},
	)
}
