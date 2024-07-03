use crate::prelude::*;
use bevy::prelude::*;

pub struct ExamplePluginText {}



impl Default for ExamplePluginText {
	fn default() -> Self { Self {} }
}


impl Plugin for ExamplePluginText {
	fn build(&self, app: &mut App) {
		app.add_plugins(ExamplePlugin)
			.add_systems(Startup, (setup, spawn_log_to_ui))
			.add_systems(Update, log_to_ui);
	}
}

fn setup(mut commands: Commands) {
	commands.spawn((DoNotSerialize, Camera2dBundle::default()));
}
