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
			.add_systems(PostStartup, clone_parent_world);
	}
}

/// Copy some types from the parent world to the router world.
fn clone_parent_world(world: &mut World) -> Result {

	let mut router = world
		.remove_resource::<Router>()
		.ok_or_else(|| bevyhow!("No Router resource found"))?;
	router.with_parent_world(world)?;
	world.insert_resource(router);
	Ok(())
}
