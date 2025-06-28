use crate::prelude::*;
use beet_bevy::prelude::AppExt;
use beet_bevy::prelude::NonSendPlugin;
use beet_template::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Config file usually located at `beet.toml`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildConfig {
	#[serde(flatten)]
	pub template_config: TemplateConfig,
	pub route_codegen: RouteCodegenConfig,
	pub client_island_codegen: ClientIslandCodegenConfig,
}

impl NonSendPlugin for BuildConfig {
	fn build(self, app: &mut App) {
		app.add_plugins(self.template_config)
			.add_non_send_plugin(self.route_codegen)
			.add_non_send_plugin(self.client_island_codegen);
	}
}
