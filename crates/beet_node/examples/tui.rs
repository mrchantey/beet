use beet_core::prelude::*;
use beet_node::prelude::style::*;
use beet_node::prelude::*;
use beet_node::*;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CharcellPlugin))
		.add_systems(Startup, setup)
		.add_systems(Update, update)
		.init_resource::<RawMode>()
		.run();
}


fn setup(mut commands: Commands) {
	let (terminal, stdio) = Terminal::new_stdout().unwrap();
	commands.spawn((
		terminal,
		stdio,
		CharcellRenderer::default(),
		FlexBox::row().column_gap(1),
		children![(
			FlexBox::row(),
			rsx! { <div>"Value: "{0}</div> },
			BoxStyle {
				border: Spacing::all(Length::Rem(1.)),
				border_top: Some(Color::srgb(1., 0., 0.)),
				border_bottom: Some(Color::srgb(0., 0.4, 1.)),
				border_left: Some(Color::srgb(0., 0.8, 0.)),
				border_right: Some(Color::srgb(1., 0.8, 0.)),
				..BoxStyle::default()
			},
		),],
	));
}


fn update(nodes: Query<&mut Value>) {
	for mut node in nodes {
		if let Value::Int(inner) = node.as_mut() {
			*node = Value::Int(*inner + 1);
		}
	}
}
