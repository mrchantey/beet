use beet_core::prelude::*;
use beet_node::prelude::style::*;
use beet_node::prelude::*;
use beet_node::*;

fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CharcellPlugin))
		.add_systems(Startup, setup)
		.run();
}


fn setup(mut commands: Commands) {
	commands.spawn((
		CharcellRenderer::default(),
		FlexBox::row().column_gap(1),
		children![(
			rsx! { "Box" },
			LayoutStyle::default().with_border(Spacing::all(Length::Rem(1.))),
			VisualStyle {
				border_top: Some(Color::srgb(1., 0., 0.)),
				border_bottom: Some(Color::srgb(0., 0.4, 1.)),
				border_left: Some(Color::srgb(0., 0.8, 0.)),
				border_right: Some(Color::srgb(1., 0.8, 0.)),
				..VisualStyle::default()
			},
		),],
	));
}
