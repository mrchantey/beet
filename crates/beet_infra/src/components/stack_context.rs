use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Debug, Clone, Get, Resource)]
pub struct StackContext {
	/// The app name, defaults to `CARGO_PKG_NAME`
	app_name: SmolStr,
	/// The deployment stage, defaults to `dev`
	stage: SmolStr,
	/// Additional parameters, some of which
	/// may be required by a config generator
	params: MultiMap<SmolStr, SmolStr>,
	/// Name of the production stage, which often receives
	/// special treatment like bucket locking and no subdomain.
	prod_stage: SmolStr,
}

impl Default for StackContext {
	fn default() -> Self {
		Self {
			app_name: std::env::var("CARGO_PKG_NAME").unwrap().into(),
			stage: "dev".into(),
			prod_stage: "prod".into(),
			params: default(),
		}
	}
}
impl StackContext {
	pub fn is_production(&self) -> bool { self.stage == self.prod_stage }

	// pub fn bucket_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
	// 	self.resource_ident("buckets", label)
	// }
	// pub fn iam_role_slug(&self, label: impl Into<SmolStr>) -> terra::Ident {
	// 	self.resource_ident("iam-roles", label)
	// }

	pub fn resource_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
		terra::Ident::new(self.app_name.clone(), self.stage.clone(), label)
	}
}
