use crate::prelude::*;
use beet_core::prelude::*;
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
