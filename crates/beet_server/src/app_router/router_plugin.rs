//! Plugin for the Beet router lifecycle
//!
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct RouterPlugin;


impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(ApplyDirectivesPlugin)
			.register_type::<MethodFilter>()
			.register_type::<Endpoint>()
			.register_type::<PathFilter>()
			.register_type::<WorkspaceConfig>()
			.register_type::<HtmlConstants>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<RenderMode>()
			.init_resource::<HtmlConstants>()
			.add_systems(Startup, insert_route_tree);
	}
}
