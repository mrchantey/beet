use beet_core::prelude::*;
use beet_ui::prelude::*;
use beet_ui::style::common_props::BackgroundColor;
use beet_ui::style::common_props::ForegroundColor;
use beet_ui::*;


fn main() {
	App::new()
		.add_plugins(CharcellPlugin)
		.insert_resource(
			RuleSet::default().with_rule(
				Rule::tag("h1")
					.with_value(
						ForegroundColor,
						Color::from(palettes::basic::GREEN),
					)
					.unwrap()
					.with_value(
						BackgroundColor,
						Color::from(palettes::basic::YELLOW),
					)
					.unwrap(),
			),
		)
		.add_systems(Startup, setup)
		.add_systems(PostUpdate, render.after(CharcellRenderSet))
		.run();
}



fn setup(mut commands: Commands) {
	commands.spawn((
		CharcellRenderer::default().halved(),
		rsx! {<h1>"hello world!"</h1>},
	));
}

fn render(query: Query<&CharcellRenderer>) -> Result {
	query.single()?.render().xprint();
	Ok(())
}
