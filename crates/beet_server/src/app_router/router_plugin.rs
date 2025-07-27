//! Plugin for the Beet router lifecycle
//!
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(TemplatePlugin)
			.register_type::<MethodFilter>()
			.register_type::<Endpoint>()
			.register_type::<RouteFilter>()
			.register_type::<WorkspaceConfig>()
			.register_type::<HtmlConstants>()
			.register_type::<TemplateFlags>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<HtmlConstants>()
			.set_runner(AppRunner::runner);
	}
}
