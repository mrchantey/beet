use std::sync::Arc;

use beet_core::prelude::*;
use bevy_ratatui::RatatuiContext;
use ratatui::prelude::Rect;
use ratatui::prelude::*;


#[derive(Clone, Component)]
pub struct TuiWidget {
	render: Arc<dyn 'static + Send + Sync + Fn(RenderWidgetContext) -> Result>,
}

pub struct RenderWidgetContext<'a> {
	pub entity: EntityWorldMut<'a>,
	/// The full area of the terminal
	pub terminal_area: Rect,
	/// A subset of the terminal area
	pub draw_area: Rect,
	pub buffer: &'a mut Buffer,
}

impl TuiWidget {
	pub fn new(
		render: impl 'static + Send + Sync + Fn(RenderWidgetContext) -> Result,
	) -> Self {
		Self {
			render: Arc::new(render),
		}
	}

	pub fn render(&mut self, cx: RenderWidgetContext) -> Result {
		(self.render)(cx)
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
				result = widget
					.render(RenderWidgetContext {
						entity: world.entity_mut(root_entity),
						// for top level, draw and terminal area are the same
						terminal_area: frame.area(),
						draw_area: frame.area(),
						buffer: frame.buffer_mut(),
					})
					.xsome();
			})?;
			result.expect("certainly assigned")
		},
	)
}
