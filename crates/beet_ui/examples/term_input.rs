use beet_core::prelude::*;
use beet_ui::prelude::*;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CharcellPlugin, RealtimeParsePlugin))
		.add_systems(Startup, setup)
		.add_systems(Update, update)
		.add_observer(on_input)
		.run();
}


fn setup(mut commands: Commands) {
	commands.spawn(
		StdioTerminal::inline(), // .with_raw_mode(true)
	);
}


fn update(nodes: Query<&mut Value>) {
	for mut node in nodes {
		if let Value::Int(inner) = node.as_mut() {
			*node = Value::Int(*inner + 1);
		}
	}
}
fn on_input(ev: On<TerminalEvent>) {
	println!("Event: {ev:?}");
}
