use bevy::prelude::*;
use heck::ToKebabCase;
use std::str::FromStr;

#[derive(Debug, Clone, Resource)]
#[cfg_attr(
	all(feature = "serde", not(target_arch = "wasm32")),
	derive(clap::Parser)
)]
pub struct InfraConfig {
	/// The name of the binary, usually one of:
	/// 1. the crate name
	/// 2. an example name
	/// 3. a test name
	#[cfg_attr(
		all(feature = "serde", not(target_arch = "wasm32")),
		arg(long, default_value_t = default_binary_name())
	)]
	pub binary_name: String,
	// The pulumi stage to use for deployments and infra resource names
	#[cfg_attr(
		all(feature = "serde", not(target_arch = "wasm32")),
		arg(long, default_value_t = default_infra_stage())
	)]
	pub stage: InfraStage,
}
fn default_binary_name() -> String {
	std::env::var("CARGO_PKG_NAME").expect("Expected CARGO_PKG_NAME to be set")
}
fn default_infra_stage() -> InfraStage {
	#[cfg(debug_assertions)]
	return InfraStage::Dev;
	#[cfg(not(debug_assertions))]
	return InfraStage::Prod;
}

/// The pulumi stage to use for deployments and infra resource names
#[derive(Debug, Clone)]
pub enum InfraStage {
	//// Development stage
	Dev,
	/// Production stage
	Prod,
	Custom(String),
}

impl Default for InfraStage {
	fn default() -> Self { default_infra_stage() }
}

impl InfraStage {
	pub fn as_str(&self) -> &str {
		match self {
			InfraStage::Dev => "dev",
			InfraStage::Prod => "prod",
			InfraStage::Custom(s) => s,
		}
	}
}

impl std::fmt::Display for InfraStage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl FromStr for InfraStage {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"dev" => Ok(InfraStage::Dev),
			"prod" => Ok(InfraStage::Prod),
			other => Ok(InfraStage::Custom(other.to_string())),
		}
	}
}
impl Default for InfraConfig {
	fn default() -> Self {
		InfraConfig {
			binary_name: default_binary_name(),
			stage: default_infra_stage(),
		}
	}
}


impl InfraConfig {
	pub fn binary_name(&self) -> &str { &self.binary_name }
	pub fn stage(&self) -> &str { self.stage.as_str() }

	pub fn default_lambda_name(&self) -> String { self.resource_name("lambda") }
	pub fn default_bucket_name(&self) -> String { self.resource_name("bucket") }

	/// Prefixes the binary name and suffixes the stage to the provided name,
	/// for example `lambda` becomes `my-site-lambda-dev`
	/// this binary-resource-stage convention must match sst config
	/// sst.config.ts -> new sst.aws.Function(`..`, {name: `THIS_FIELD` }),
	pub fn resource_name(&self, name: &str) -> String {
		let binary_name = self.binary_name.to_kebab_case();
		let stage = self.stage.as_str();
		format! {"{binary_name}-{name}-{stage}"}
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		InfraConfig::default()
			.resource_name("lambda")
			.xpect()
			.to_be("beet-core-lambda-dev");
	}
}
