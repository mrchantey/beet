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
#[cfg(feature = "aws")]
use beet_net::prelude::*;

/// Irreversibly remove the backend, destroying the tofu state for **all applications**.
pub async fn dangerously_destroy_backend(backend: &StackBackend) -> Result {
	match backend {
		StackBackend::Local(local) => {
			fs_ext::remove_async(local.path()).await?;
		}
		#[allow(unused)]
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

const NOT_FOUND: &str = r#"
It looks like opentofu is not installed, this is required for deploying infrastructure.
Please install and try again
https://opentofu.org/docs/intro/install
"#;

fn tofu_process<'a>() -> Process<'a> {
	Process::new("tofu").with_not_found(NOT_FOUND)
}


/// Export the provider schema based on `./providers.tf.json`
pub async fn export_schema(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["providers", "schema", "-json"])
		.run_async_stdout()
		.await
}

/// Initialize an opentofu directory, using the `./providers.tf.json`.
/// Uses `-reconfigure` to allow backend type changes without migrating state.
pub async fn init(dir: &AbsPathBuf, force: bool) -> Result {
	let args = if force {
		vec!["init", "-reconfigure"]
	} else {
		vec!["init"]
	};

	tofu_process()
		.with_cwd(dir)
		.with_args(&args)
		.run_async()
		.await?;
	Ok(())
}

/// Validates the opentofu file, ie the `main.tf.json`
pub async fn validate(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["validate", "-json"])
		.run_async_stdout()
		.await
}

/// Show execution plan
pub async fn plan(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["plan"])
		.run_async_stdout()
		.await
}

/// Apply the execution plan
pub async fn apply(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["apply", "-auto-approve"])
		.run_async_stdout()
		.await
}

/// Show the current state
pub async fn show(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["show"])
		.run_async_stdout()
		.await
}

/// List all resources in the state
pub async fn list(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["state", "list"])
		.run_async_stdout()
		.await
}

/// Remove a resource from the state
pub async fn remove(dir: &AbsPathBuf, resource: &str) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["state", "rm", resource])
		.run_async_stdout()
		.await
}

/// Destroy infrastructure
pub async fn destroy(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["destroy", "-auto-approve"])
		.run_async_stdout()
		.await
}

/// Destroy infrastructure, bypassing any stale state locks.
/// Used only by `force_destroy` recovery paths where we know
/// no concurrent operation is active.
pub async fn destroy_force(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir)
		.with_args(&["destroy", "-auto-approve", "-lock=false"])
		.run_async_stdout()
		.await
}
