use beet::prelude::*;
use bevy::prelude::*;

use crate::prelude::BundlePlaceholder;


pub fn camera_2d(mut commands: Commands) {
	commands.spawn(BundlePlaceholder::Camera2d);
}
pub fn camera_3d(mut commands: Commands) {
	commands.spawn(BundlePlaceholder::Camera3d);
}
pub fn beet_debug(mut commands: Commands) {
	commands.insert_resource(BeetDebugConfig::default());
}
