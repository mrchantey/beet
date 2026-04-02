use crate::types::TerraBackend;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

#[derive(Get, Component)]
pub struct Stack {
	/// The backend configuration for storing the state of this stack,
	/// defaults to a local backend at `./infra-state`
	backend: StackBackend,
}


impl Stack {
	pub fn new(backend: impl Into<StackBackend>) -> Self {
		Self {
			backend: backend.into(),
		}
	}
}

impl Default for Stack {
	fn default() -> Self { Self::new(S3Backend::default()) }
}

/// Strategy for maintaining the Terraform state for this stack.
/// https://opentofu.org/docs/language/settings/backends/configuration/
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

impl TerraBackend for StackBackend {
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
#[derive(Serialize, Deserialize)]
pub struct LocalBackend {
	path: AbsPathBuf,
}

impl Default for LocalBackend {
	fn default() -> Self {
		Self {
			path: WsPathBuf::new(STATE_RESOURCE_NAME).into(),
		}
	}
}

impl TerraBackend for LocalBackend {
	fn backend_type(&self) -> &'static str { "local" }
	fn to_backend_json(&self) -> Value {
		json!({ "path": self.path.to_string() })
	}
}

const STATE_RESOURCE_NAME: &str = "tofu-state";

/// S3 backend for remote state storage.
/// https://opentofu.org/docs/language/settings/backends/s3/
#[derive(SetWith, Serialize, Deserialize)]
pub struct S3Backend {
	/// The S3 bucket containing the state file.
	bucket: String,
	/// Path to the state file within the bucket (e.g. `"env/prod/terraform.tfstate"`).
	key: String,
	/// AWS region where the bucket lives.
	region: AwsRegion,
	/// Enable OpenTofu's native S3 lockfile.
	use_lockfile: bool,
}

impl Default for S3Backend {
	fn default() -> Self {
		Self {
			bucket: String::new(),
			key: "terraform.tfstate".to_string(),
			region: AwsRegion::default(),
			use_lockfile: true,
		}
	}
}

impl TerraBackend for S3Backend {
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


#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize)]
pub enum AwsRegion {
	#[default]
	#[serde(rename = "us-east-1")]
	UsEast1,
	#[serde(rename = "us-west-2")]
	UsWest2,
}

impl std::fmt::Display for AwsRegion {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AwsRegion::UsEast1 => write!(f, "us-east-1"),
			AwsRegion::UsWest2 => write!(f, "us-west-2"),
		}
	}
}
