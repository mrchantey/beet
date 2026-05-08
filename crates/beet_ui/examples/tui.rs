use beet_core::prelude::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use beet_ui::*;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CharcellPlugin))
		.add_systems(Startup, setup)
		.add_systems(Update, update)
		.add_observer(on_input)
		.run();
}

fn def_count() -> (TokenDefinition, SetToken<i32>) { token(0) }

fn setup(mut commands: Commands) {
	let (count, _set_cout) = def_count();

	commands.spawn((
		StdioTerminal::default(),
		CharcellRenderer::default(),
		FlexBox::row().column_gap(1),
		children![(
			FlexBox::row(),
			rsx! { <div>"Value: "{(0,count)}</div> },
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


fn update(
	mut commands: Commands,
	query: Query<Entity, With<TokenStore>>,
) -> Result {
	let entity = query.single()?;
	let (_, set) = def_count();
	commands.entity(entity).queue(set.set(2));
	Ok(())
}
// fn update(nodes: Query<&mut Value>) {
// 	for mut node in nodes {
// 		if let Value::Int(inner) = node.as_mut() {
// 			*node = Value::Int(*inner + 1);
// 		}
// 	}
// }


fn on_input(ev: On<TerminalEvent>, mut _commands: Commands) {
	match ev.event() {
		TerminalEvent::Key(_key_press) => {}
		TerminalEvent::Mouse(_mouse) => {}
		_ => {}
	}
}
