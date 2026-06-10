use beet_core::prelude::*;
use beet_ui::prelude::*;
use bevy::input::keyboard::KeyboardInput;


fn main() {
	App::new()
		.add_plugins((MinimalPlugins, CharcellTuiPlugin))
		.add_systems(Startup, setup)
		.add_systems(Update, (update, log_input))
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

/// Terminal input now flows through bevy's unified `KeyboardInput` messages.
fn log_input(mut keys: MessageReader<KeyboardInput>) {
	for key in keys.read() {
		cross_log!("Key: {key:?}");
	}
}
