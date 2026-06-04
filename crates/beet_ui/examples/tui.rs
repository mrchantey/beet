use beet_core::prelude::*;
use beet_ui::prelude::style::*;
use beet_ui::prelude::*;
use beet_ui::*;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CharcellPlugin, DocumentPlugin))
		.add_systems(Startup, setup)
		.add_systems(Update, update)
		.add_observer(on_input)
		.run();
}

fn count_field() -> FieldRef { FieldRef::new("count") }

fn setup(mut commands: Commands) {
	commands.spawn((
		StdioTerminal::default(),
		DoubleBuffer::default(),
		Document::new(val!({ "count": 0i64 })),
		LayoutStyle::flex_row().column_gap(1),
		children![((
			LayoutStyle::flex_row(),
			rsx_direct! { <div>"Value: "{(Value::default(), count_field())}</div> },
			BoxStyle {
				border: Spacing::all(Length::Rem(1.)),
				border_top: Some(Color::srgb(1., 0., 0.)),
				border_bottom: Some(Color::srgb(0., 0.4, 1.)),
				border_left: Some(Color::srgb(0., 0.8, 0.)),
				border_right: Some(Color::srgb(1., 0.8, 0.)),
				..BoxStyle::default()
			},
		),)],
	));
}


fn update(
	terminals: Query<Entity, With<DoubleBuffer>>,
	mut query: DocumentQuery,
) -> Result {
	let entity = terminals.single()?;
	query.with_field(entity, &count_field(), |value| {
		*value = Value::Int(value.as_i64().unwrap_or(0) + 1);
	})
}

fn on_input(ev: On<TerminalEvent>, mut _commands: Commands) {
	match ev.event() {
		TerminalEvent::Key(_key_press) => {}
		TerminalEvent::Mouse(_mouse) => {}
		_ => {}
	}
}
