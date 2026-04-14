use crate::bindings::aws;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// Strategy for maintaining the Terraform state for this stack.
/// By default, the state for each stack is
/// stored in an individual directory in a shared state bucket.
/// https://opentofu.org/docs/language/settings/backends/configuration/
#[derive(Debug, Clone, PartialEq, Eq)]
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
	pub fn to_json(&self, key: &str) -> Value {
		match self {
			Self::Local(b) => b.to_json(&key),
			Self::S3(b) => b.to_json(&key),
		}
	}

	#[cfg(feature = "aws")]
	pub fn provider(&self) -> Box<dyn BucketProvider> {
		match self {
			Self::S3(s3) => s3.provider().box_clone(),
			Self::Local(local) => local.provider().box_clone(),
		}
	}

	/// Ensure the backend exists, creating the directory or s3 bucket if it doesn't exist.
	pub async fn ensure_exists(&self) -> Result {
		self.provider().bucket_try_create().await
	}

	/// Clear stale lock files if the backend supports it.
	pub fn clear_stale_locks(&self) {
		match self {
			Self::Local(local) => local.clear_stale_locks(),
			Self::S3(_) => {
				// S3 lock management is handled server-side by OpenTofu's native lockfile mechanism
			}
		}
	}

	/// Remove this backend bucket if its empty
	pub async fn remove_if_empty(&self) -> Result {
		let provider = self.provider();
		if provider.bucket_is_empty().await? {
			provider.bucket_remove().await?;
		}
		Ok(())
	}
}

/// Local filesystem backend, defaults to `.beet/infra`
/// https://opentofu.org/docs/language/settings/backends/local/
#[derive(Debug, Clone, PartialEq, Eq, Get)]
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
	fn to_json(&self, key: &str) -> Value {
		// Use the absolute path string directly. AbsPathBuf's Serialize impl
		// converts to a workspace-relative path, but terraform's local backend
		// resolves relative paths from the tofu working directory, not the
		// workspace root.
		let state_path = self.path.join(key).to_string();
		json!({"local":{ "path": state_path }})
	}
	pub fn provider(&self) -> FsBucket { FsBucket::new(self.path.clone()) }
	/// Remove stale `.*.lock.info` files left by interrupted tofu processes.
	pub fn clear_stale_locks(&self) {
		if let Ok(entries) =
			std::fs::read_dir(self.path.as_ref() as &std::path::Path)
		{
			for entry in entries.flatten() {
				let name = entry.file_name();
				let name = name.to_string_lossy();
				if name.starts_with('.') && name.ends_with(".lock.info") {
					std::fs::remove_file(entry.path()).ok();
				}
			}
		}
	}
}

const DEFAULT_STATE_NAME: &str = "beet-state";

/// S3 backend for remote state storage.
/// https://opentofu.org/docs/language/settings/backends/s3/
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith)]
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
	pub fn provider(&self) -> beet_net::prelude::S3Bucket {
		beet_net::prelude::S3Bucket::new(
			self.bucket.clone(),
			self.region.clone(),
		)
	}
}

impl Default for S3Backend {
	fn default() -> Self {
		Self {
			bucket: DEFAULT_STATE_NAME.into(),
			// State bucket lives in us-east-1 as a stable singleton,
			// independent of the managed-resource region.
			region: aws::region::US_EAST_1.into(),
			use_lockfile: true,
		}
	}
}

impl S3Backend {
	fn to_json(&self, key: &str) -> Value {
		json!({
			"s3": {
				"bucket": self.bucket,
				"key": key,
				"region": self.region.to_string(),
				"use_lockfile": self.use_lockfile,
			}
		})
	}
}
