use crate::prelude::*;
use bevy::prelude::*;


pub struct EmbyPlugin;

impl Plugin for EmbyPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(emote_agent_plugin)
			.add_systems(Startup, setup)
			.add_observer(crate::scenes::add_phone_render_texture_to_arm);
	}
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.insert_resource(EmojiMap::new(&asset_server));
}
