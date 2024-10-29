use crate::prelude::*;
use bevy::prelude::*;


pub struct EmbyPlugin;

impl Plugin for EmbyPlugin {
	fn build(&self, app: &mut App) {
		/*-*/

		app.add_plugins(emote_agent_plugin);
	}
}
