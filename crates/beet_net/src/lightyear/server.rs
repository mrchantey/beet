use bevy::prelude::*;
use lightyear::prelude::server::*;

// Plugin for server-specific logic
pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
	fn build(&self, app: &mut App) { app.add_systems(Startup, init); }
}

fn init(mut commands: Commands) { commands.start_server(); }
