use beet_flow::prelude::*;
use bevy::prelude::*;

pub fn beet_debug(mut commands: Commands) {
	commands.insert_resource(BeetDebugConfig::default());
}
