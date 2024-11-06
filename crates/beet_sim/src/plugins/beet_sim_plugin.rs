use crate::prelude::*;
use bevy::prelude::*;

pub struct BeetSimPlugin;

impl Plugin for BeetSimPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((emoji_plugin, walk_plugin, stat_plugin));
	}
}
