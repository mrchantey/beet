use beet_core::prelude::*;



pub struct ServerPlugin;

impl Plugin for ServerPlugin {
	fn build(&self, app: &mut App) { app.init_plugin(AsyncPlugin); }
}
