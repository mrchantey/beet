use super::*;
use bevy::prelude::*;



pub struct ChannelEventPlugin {
	spawn_handler: SpawnHandler,
}


impl Plugin for ChannelEventPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.spawn_handler.clone())
			.add_systems(PreUpdate, SpawnHandler::system);
	}
}
