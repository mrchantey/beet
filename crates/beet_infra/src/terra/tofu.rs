//! Module for interacting with tofu
//!
//! ## Architecture
//!
//! The default approach is a single state backend, ie a directory or s3 bucket,
//! with each stack (app-stage pair) having its own state,
//! ie
//! `beet-state/beet--dev/..state`
//!
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Ensure the backend exists, creating the directory or s3 bucket if it doesn't exist.
pub async fn ensure_backend_exists(backend: &StackBackend) -> Result {
	match backend {
		StackBackend::Local(local) => {
			fs_ext::create_dir_all_async(local.path()).await?;
		}
		StackBackend::S3(s3) => {
			cfg_if! {
				if #[cfg(feature = "aws")] {
					s3.provider()
						.bucket_try_create().await?;
				} else {
					bevybail!("S3 backend support requires the `aws` feature flag")
				}
			}
		}
	}
	Ok(())
}

/// Irreversibly remove the backend, destroying the tofu state for **all applications**.
pub async fn dangerously_destroy_backend(backend: &StackBackend) -> Result {
	match backend {
		StackBackend::Local(local) => {
			fs_ext::remove_async(local.path()).await?;
		}
		StackBackend::S3(s3) => {
			cfg_if! {
				if #[cfg(feature = "aws")] {
					s3.provider()
						.bucket_remove().await?;
				} else {
					bevybail!("S3 backend support requires the `aws` feature flag")
				}
			}
		}
	}
	Ok(())
}


/// Export the provider schema based on `./providers.tf.json`
pub async fn export_schema(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["providers", "schema", "-json"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// Initialize an opentofu directory, using the `./providers.tf.json`
pub async fn init(dir: &AbsPathBuf) -> Result {
	Process::new("tofu")
		.with_args(&["init"])
		.with_cwd(dir)
		.run_async()
		.await?;
	Ok(())
}

/// Validates the opentofu file, ie the `main.tf.json`
pub async fn validate(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["validate", "-json"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// Show execution plan
pub async fn plan(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["plan"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// Apply the execution plan
pub async fn apply(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["apply"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// Show the current state
pub async fn show(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["show"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// List all resources in the state
pub async fn list(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["state", "list"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// Remove a resource from the state
pub async fn remove(dir: &AbsPathBuf, resource: &str) -> Result<String> {
	Process::new("tofu")
		.with_args(&["state", "rm", resource])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}

/// Destroy infrastructure
pub async fn destroy(dir: &AbsPathBuf) -> Result<String> {
	Process::new("tofu")
		.with_args(&["destroy"])
		.with_cwd(dir)
		.run_async_stdout()
		.await
}
