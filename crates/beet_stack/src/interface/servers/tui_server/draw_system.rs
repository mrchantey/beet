use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QuerySingleError;
use bevy_ratatui::*;
use ratatui::prelude::*;
use ratatui::widgets;

pub(super) fn draw_system(
	mut context: ResMut<RatatuiContext>,
	query: Query<Entity, With<CurrentCard>>,
) -> Result {
	let card = query.single()?;

	context.draw(render)?;

	Ok(())
}


#[derive(Component)]
pub struct ViewRoot;

pub(super) fn diff_view(
	insert: On<Insert, CurrentCard>,
	mut diff_query: DiffQuery,
) -> Result {
	diff_query.diff(insert.entity)?;
	Ok(())
}

#[derive(SystemParam)]
pub(super) struct DiffQuery<'w, 's> {
	root: Query<'w, 's, Entity, With<ViewRoot>>,
	state: Query<
		'w,
		's,
		(
			Option<&'static Children>,
			Option<&'static Title>,
			Option<&'static Paragraph>,
		),
	>,
	commands: Commands<'w, 's>,
}

impl DiffQuery<'_, '_> {
	fn diff(&mut self, card: Entity) -> Result {
		let view_root = match self.root.single() {
			Ok(entity) => entity,
			Err(QuerySingleError::MultipleEntities(_)) => {
				bevybail!("Multiple entities with ViewRoot component found")
			}
			Err(QuerySingleError::NoEntities(_)) => {
				self.commands.spawn_empty().id()
			}
		};
		self.diff_inner(card, view_root)?;
		Ok(())
	}

	fn diff_inner(
		&mut self,
		interface_entity: Entity,
		view_entity: Entity,
	) -> Result {
		let (children, title, paragraph) = self.state.get(interface_entity)?;
		if let Some(title) = title {}
		if let Some(children) =
			children.map(|children| children.iter().collect::<Vec<_>>())
		{
			for child in children {
				self.diff_inner(child, view_entity)?;
			}
		}
		Ok(())
	}
}

#[derive(Copy, Clone)]
struct DiffState {
	interface_entity: Entity,
	view_entity: Entity,
	title: bool,
	paragraph: bool,
	emphasize: bool,
}


fn render(frame: &mut Frame) {
	let title = Line::from("Ratatui Simple Template")
		.bold()
		.blue()
		.centered();
	let text = "Hello, Ratatui!\n\n\
        Created using https://github.com/ratatui/templates\n\
        Press `Esc`, `Ctrl-C` or `q` to stop running.";
	frame.render_widget(
		widgets::Paragraph::new(text)
			.block(widgets::Block::bordered().title(title))
			.centered(),
		frame.area(),
	)
}
