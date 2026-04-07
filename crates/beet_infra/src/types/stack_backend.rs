use crate::bindings::aws;
use crate::terra;
use beet_core::prelude::*;
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

impl terra::Backend for StackBackend {
	fn backend_type(&self) -> &'static str {
		match self {
			StackBackend::Local(b) => b.backend_type(),
			StackBackend::S3(b) => b.backend_type(),
		}
	}
	fn to_backend_json(&self) -> Value {
		match self {
			StackBackend::Local(b) => b.to_backend_json(),
			StackBackend::S3(b) => b.to_backend_json(),
		}
	}
}

/// Local filesystem backend.
/// https://opentofu.org/docs/language/settings/backends/local/
#[derive(Debug, Clone, Get)]
pub struct LocalBackend {
	path: AbsPathBuf,
}

impl Default for LocalBackend {
	fn default() -> Self {
		Self {
			path: WsPathBuf::new(DEFAULT_STATE_NAME).into(),
		}
	}
}

impl terra::Backend for LocalBackend {
	fn backend_type(&self) -> &'static str { "local" }
	fn to_backend_json(&self) -> Value {
		json!({ "path": self.path.to_string() })
	}
}

const DEFAULT_STATE_NAME: &str = "beet-state";

/// S3 backend for remote state storage.
/// https://opentofu.org/docs/language/settings/backends/s3/
#[derive(Debug, Clone, Get, SetWith)]
pub struct S3Backend {
	/// The S3 bucket containing the state file, defaults to `beet-state`
	bucket: SmolStr,
	/// Path to the state file within the bucket (e.g. `"env/prod/terraform.tfstate"`).
	key: SmolStr,
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
			key: "terraform.tfstate".into(),
			region: aws::region::DEFAULT.into(),
			use_lockfile: true,
		}
	}
}

impl terra::Backend for S3Backend {
	fn backend_type(&self) -> &'static str { "s3" }
	fn to_backend_json(&self) -> Value {
		json!({
			"bucket": self.bucket,
			"key": self.key,
			"region": self.region.to_string(),
			"use_lockfile": self.use_lockfile,
		})
	}
}
