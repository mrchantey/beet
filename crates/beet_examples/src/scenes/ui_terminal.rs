use crate::prelude::*;
use bevy::prelude::*;

pub fn ui_terminal_input(commands: Commands) {
	spawn_ui_terminal(commands, true);
}
pub fn ui_terminal(commands: Commands) { spawn_ui_terminal(commands, false); }
