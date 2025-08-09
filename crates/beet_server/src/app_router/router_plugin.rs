//! Plugin for the Beet router lifecycle
//!
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct RouterPlugin;


impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<MethodFilter>()
			.register_type::<Endpoint>()
			.register_type::<PathFilter>()
			.register_type::<WorkspaceConfig>()
			.register_type::<HtmlConstants>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<RenderMode>()
			.init_resource::<HtmlConstants>()
			.add_systems(Startup, clone_parent_world);
	}
}


/// Copy some types from the parent world to the router world.
fn clone_parent_world(world: &mut World) -> Result {
	if let Some(mut router) = world.remove_resource::<Router>() {
		let render_mode = world.resource::<RenderMode>().clone();


		router.add_plugin(move |app: &mut App| {
			app.insert_resource(render_mode.clone());
		});
		router.validate()?;
		world.insert_resource(router);
	}
	Ok(())
}
