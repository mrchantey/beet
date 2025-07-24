//! Plugin for the Beet router lifecycle
//!
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
pub struct AppRouterPlugin;

impl Plugin for AppRouterPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(TemplatePlugin)
			.register_type::<RouteSegment>()
			.register_type::<WorkspaceConfig>()
			.register_type::<HtmlConstants>()
			.register_type::<TemplateFlags>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<HtmlConstants>()
			.set_runner(AppRunner::runner);
	}
}