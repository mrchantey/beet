use crate::prelude::*;
use beet_core::prelude::*;
use ratatui::prelude::Direction;
use ratatui::prelude::Layout as RatLayout;
use ratatui::prelude::*;


/// Horizontal not yet supported
#[derive(Default, Clone, Component)]
#[require(TuiWidget=widget())]
pub struct Layout {
	direction: Direction,
}
impl Layout {
	pub fn vertical() -> Self {
		Self {
			direction: Direction::Vertical,
		}
	}
}

fn widget() -> TuiWidget {
	TuiWidget::new(Constraint::Length(3), |cx| {
		let Layout { direction } = cx
			.entity
			.get::<Layout>()
			.ok_or_else(|| {
				bevyhow!(
					"Layout component missing from entity {:?}",
					cx.entity.id()
				)
			})?
			.clone();

		let id = cx.entity.id();
		let world = cx.entity.into_world_mut();

		let children = world
			.entity(id)
			.get::<Children>()
			.map(|children| {
				children
					.into_iter()
					.filter_map(|child| {
						world
							.entity(*child)
							.get::<TuiWidget>()
							.cloned()
							.map(|widget| (*child, widget))
					})
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();

		let rat_layout = RatLayout::new(
			direction,
			children.iter().map(|(_, child)| child.constraint()),
		);

		let areas = rat_layout.split(cx.draw_area);

		for ((child_entity, mut child_widget), draw_area) in
			children.into_iter().zip(areas.into_iter())
		{
			child_widget.render(RenderWidgetContext {
				entity: world.entity_mut(child_entity),
				terminal_area: cx.terminal_area,
				draw_area: *draw_area,
				buffer: cx.buffer,
			})?;
		}
		Ok(())
	})
}
