// use beet::exports::ratatui::text;
// use beet::exports::ratatui::widgets;
// use beet::exports::ratatui::widgets::Widget;
use beet::prelude::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, TuiPlugin2))
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut _commands: Commands) {
	// commands.spawn(EntityWidget::new(render));
}


// fn render(mut cx: TuiRenderContext) -> Result {
// 	text::Span::raw("hello world").render(cx.draw_area, cx.buffer);
// 	Ok(())
// }
