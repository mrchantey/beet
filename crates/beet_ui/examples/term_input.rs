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
	// keyboard-only demo: disable mouse reporting so the terminal is never put
	// into mouse-tracking mode and the example emits only key events, not a
	// stream of mouse escape sequences.
	commands.spawn(
		StdioTerminal::inline()
			.with_config(TerminalConfig::inline().with_enable_mouse(false)),
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
		info!("Key: {key:?}");
	}
}
