//! Plugin for the Beet router lifecycle
//!
use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_net::prelude::*;

#[derive(Default)]
pub struct RouterPlugin;


impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<MethodFilter>()
			.register_type::<HttpMethod>()
			.register_type::<CacheStrategy>()
			.register_type::<PathFilter>()
			.register_type::<WorkspaceConfig>()
			.register_type::<HtmlConstants>()
			.init_resource::<WorkspaceConfig>()
			// .init_resource::<PackageConfig>()
			.init_resource::<RenderMode>()
			.init_resource::<HtmlConstants>()
			.add_systems(PostStartup, Router::clone_parent_world);
	}
}
