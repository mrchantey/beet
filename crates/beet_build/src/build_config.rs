use crate::prelude::*;
use beet_template::prelude::*;
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
