use std::path::PathBuf;

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
	params: MultiMap<String, String>,
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

	pub fn bucket_slug(&self, key: impl Into<SmolStr>) -> Slug {
		self.resource_slug("buckets", key)
	}
	pub fn resource_slug(
		&self,
		resource_kind: impl Into<SmolStr>,
		key: impl Into<SmolStr>,
	) -> Slug {
		Slug::new(vec![
			self.app_name.clone(),
			self.stage.clone(),
			resource_kind.into(),
			key.into(),
		])
	}
}

pub trait ToTerraConfig {
	fn to_terra_config(&self, cx: &StackContext) -> Result<TerraConfig>;
}

/// Resource identifier, usually comprised of a tuple of strings,
/// ie app-name, stage, resource name
pub struct Slug(Vec<SmolStr>);
impl Slug {
	pub fn new(parts: impl IntoIterator<Item = impl Into<SmolStr>>) -> Self {
		Self(parts.into_iter().map(|s| s.into().into()).collect())
	}
	/// Converts the slug to alphanumeric and dashes
	/// ie `my-app, prod, buckets, html` becomes:
	/// `my-app--prod--buckets--html`
	pub fn to_kebab_case(&self) -> String {
		use heck::ToKebabCase;
		self.0.join("--").to_kebab_case()
	}
	/// Converts the slug to alphanumeric and underscores
	/// ie `my-app, prod, buckets, html` becomes:
	/// `my_app__prod__buckets__html`
	pub fn to_snake_case(&self) -> String {
		use heck::ToSnakeCase;
		self.0.join("__").to_snake_case()
	}

	/// Converts to kebab case path
	pub fn to_path(&self) -> PathBuf {
		use heck::ToKebabCase;
		self.0.iter().fold(PathBuf::new(), |mut path, part| {
			path.push(part.to_kebab_case());
			path
		})
	}
}
