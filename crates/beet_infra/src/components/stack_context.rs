use beet_core::prelude::*;

use crate::prelude::ConfigExporter;

#[derive(Debug, Clone, Get)]
pub struct StackContext {
	/// The app name, defaults to `CARGO_PKG_NAME`
	app_name: String,
	/// The deployment stage, defaults to `dev`
	stage: String,
	/// Additional parameters, some of which
	/// may be required by a config generator
	params: MultiMap<String, String>,
}

impl Default for StackContext {
	fn default() -> Self {
		Self {
			app_name: env!("CARGO_PKG_NAME").into(),
			stage: "dev".into(),
			params: default(),
		}
	}
}
impl StackContext {
	pub fn is_production(&self) -> bool {
		self.stage == "prod" || self.stage == "production"
	}
}

pub trait ToTerraConfig {
	fn to_terra_config(&self, cx: &StackContext) -> Result<ConfigExporter>;
}
