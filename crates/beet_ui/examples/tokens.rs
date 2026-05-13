use beet_core::prelude::*;
use beet_ui::prelude::*;
use beet_ui::style::TextAlign;
use beet_ui::style::TextStyle;
use beet_ui::style::common_props::BackgroundColor;
use beet_ui::style::common_props::ForegroundColor;
use beet_ui::style::common_props::TextAlignProp;
use beet_ui::style::common_props::TextStyleProp;
use beet_ui::*;


fn main() {
	App::new()
		.add_plugins(CharcellPlugin)
		.insert_resource(
			RuleSet::default().with_rule(
				Rule::tag("h1")
					.with_value(TextAlignProp, TextAlign::Center)
					.with_value(TextStyleProp, TextStyle::BOLD)
					.with_value(
						ForegroundColor,
						palettes::tailwind::FUCHSIA_400,
					)
					.with_value(BackgroundColor, palettes::tailwind::GREEN_900),
			),
		)
		.add_systems(Startup, setup)
		.add_systems(PostUpdate, render.after(CharcellRenderSet))
		.run();
}



fn setup(mut commands: Commands) {
	commands.spawn((
		DoubleBuffer::new_half_terminal(),
		rsx! {<h1>"hello world!"</h1>},
	));
}

fn render(query: Query<&DoubleBuffer>) -> Result {
	query.single()?.render().xprint();
	Ok(())
}
