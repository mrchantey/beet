use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;


pub fn beet_debug(mut commands: Commands) {
	commands.insert_resource(BeetDebugConfig::default());
}

pub fn ui_terminal_input(commands: Commands) {
	spawn_ui_terminal(commands, true);
}
pub fn ui_terminal(commands: Commands) { spawn_ui_terminal(commands, false); }
