use crate::bindings::aws;
use crate::terra;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// Strategy for maintaining the Terraform state for this stack.
/// By default, the state for each stack is
/// stored in an individual directory in a shared state bucket.
/// https://opentofu.org/docs/language/settings/backends/configuration/
#[derive(Debug, Clone)]
pub enum StackBackend {
	Local(LocalBackend),
	S3(S3Backend),
}

impl From<LocalBackend> for StackBackend {
	fn from(b: LocalBackend) -> Self { StackBackend::Local(b) }
}
impl From<S3Backend> for StackBackend {
	fn from(b: S3Backend) -> Self { StackBackend::S3(b) }
}
impl StackBackend {
	/// Create the body of the opentofu "backend" field
	pub fn to_json(&self, ident: &terra::Ident) -> Value {
		match self {
			Self::Local(b) => b.to_json(&ident),
			Self::S3(b) => b.to_json(&ident),
		}
	}

	#[cfg(feature = "aws")]
	pub fn provider(&self) -> Box<dyn BucketProvider> {
		match self {
			Self::S3(s3) => s3.provider().box_clone(),
			Self::Local(local) => local.provider().box_clone(),
		}
	}
}

/// Local filesystem backend, defaults to `.beet/infra`
/// https://opentofu.org/docs/language/settings/backends/local/
#[derive(Debug, Clone, Get)]
pub struct LocalBackend {
	/// The path on the local filesystem where the state file will be stored, defaults to `.beet/infra`.
	path: AbsPathBuf,
}

impl Default for LocalBackend {
	fn default() -> Self {
		Self {
			path: WsPathBuf::new(".beet/infra").into(),
		}
	}
}
impl LocalBackend {
	fn to_json(&self, ident: &terra::Ident) -> Value {
		let ident = ident.primary_identifier();
		json!({"local":{ "path": self.path.join(ident) }})
	}
	pub fn provider(&self) -> FsBucketProvider {
		FsBucketProvider::new(self.path.clone())
	}
}

const DEFAULT_STATE_NAME: &str = "beet-state";

/// S3 backend for remote state storage.
/// https://opentofu.org/docs/language/settings/backends/s3/
#[derive(Debug, Clone, Get, SetWith)]
pub struct S3Backend {
	/// The S3 bucket containing the state file, defaults to `beet-state`
	bucket: SmolStr,
	/// AWS region where the bucket lives.
	region: SmolStr,
	/// Enable OpenTofu's native S3 lockfile.
	use_lockfile: bool,
}

impl S3Backend {
	#[cfg(feature = "aws")]
	pub fn provider(&self) -> beet_net::prelude::S3Provider {
		beet_net::prelude::S3Provider::new(
			self.bucket.clone(),
			self.region.clone(),
		)
	}
}

impl Default for S3Backend {
	fn default() -> Self {
		Self {
			bucket: DEFAULT_STATE_NAME.into(),
			region: aws::region::DEFAULT.into(),
			use_lockfile: true,
		}
	}
}

impl S3Backend {
	fn to_json(&self, ident: &terra::Ident) -> Value {
		json!({
			"s3": {
				"bucket": self.bucket,
				"key": ident.primary_identifier(),
				"region": self.region.to_string(),
				"use_lockfile": self.use_lockfile,
			}
		})
	}
}
