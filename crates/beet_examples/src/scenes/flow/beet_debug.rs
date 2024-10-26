use crate::beet::prelude::*;
use bevy::prelude::*;

pub fn beet_debug(mut commands: Commands) {
	commands.insert_resource(BeetDebugConfig::default());
}
pub fn beet_debug_start_and_stop(mut commands: Commands) {
	commands.insert_resource(BeetDebugConfig::start_and_stop());
}
