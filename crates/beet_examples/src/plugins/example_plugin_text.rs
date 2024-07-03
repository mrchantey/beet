use crate::prelude::*;
use bevy::prelude::*;
use bevy::ui::UiSystem;

pub struct ExamplePluginText {}



impl Default for ExamplePluginText {
	fn default() -> Self { Self {} }
}


impl Plugin for ExamplePluginText {
	fn build(&self, app: &mut App) {
		app.add_plugins(ExamplePlugin)
			.add_systems(Startup, (setup, spawn_log_to_ui))
			.add_systems(Update, log_to_ui)
			.add_systems(
				PostUpdate,
				(scroll_to_bottom_on_resize, scroll_to_bottom_on_append)
					.after(UiSystem::Layout),
			);
	}
}

fn setup(mut commands: Commands) {
	commands.spawn(Camera2dBundle::default());
}
