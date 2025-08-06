use bevy::prelude::*;

/// Cross-service storage bucket representation.
/// To constrain to a specific service include the Aws or CloudFlare tags.
pub struct Bucket {
	/// The resource name of the bucket.
	pub name: String,
	pub region: Option<String>,
}


/// A very light wrapper around sst, adapted to a few beet specific conventions
/// like ensuring commands are run in the `infra` directory.
#[derive(Debug, Clone, Resource)]
#[cfg_attr(all(feature = "serde", not(target_arch = "wasm32")), derive(clap::Parser))]
pub struct InfraConfig {
	/// The default stage for development.
	#[cfg_attr(all(feature = "serde", not(target_arch = "wasm32")), arg(long, default_value = "dev"))]
	pub dev_stage: String,
	/// The default stage for production.
	#[cfg_attr(all(feature = "serde", not(target_arch = "wasm32")), arg(long, default_value = "prod"))]
	pub prod_stage: String,
	/// Optionally specify the sst stage name used, which will otherwise be inferred
	/// from debug/release build, defaulting to `dev` or `prod`.
	#[cfg_attr(all(feature = "serde", not(target_arch = "wasm32")), arg(long))]
	pub stage: Option<String>,
}

impl Default for InfraConfig {
	fn default() -> Self {
		InfraConfig {
			dev_stage: "dev".to_string(),
			prod_stage: "prod".to_string(),
			stage: None,
		}
	}
}
