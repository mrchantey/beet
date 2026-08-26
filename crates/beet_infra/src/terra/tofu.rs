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
#[cfg(feature = "aws_sdk")]
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
				if #[cfg(feature = "aws_sdk")] {
					s3.provider()
						.store_remove().await?;
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

fn tofu_process() -> ChildProcess {
	ChildProcess::new("tofu").with_not_found(NOT_FOUND)
}

/// Render `vars` as `-var key=value` pairs, the form every state-touching
/// tofu subcommand accepts. Used to thread a stack's [`StateEncryption`]
/// passphrase (and any other required vars) through without ever writing
/// them into `main.tf.json`.
fn var_args(vars: &[(SmolStr, SmolStr)]) -> Vec<SmolStr> {
	let mut args = Vec::with_capacity(vars.len() * 2);
	for (key, value) in vars {
		args.push("-var".into());
		args.push(format!("{key}={value}").into());
	}
	args
}

/// Export the provider schema based on `./providers.tf.json`
pub async fn export_schema(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(["providers", "schema", "-json"])
		.run_async_stdout()
		.await
}

/// Initialize an opentofu directory, using the `./providers.tf.json`.
/// Always passes `-reconfigure` so the shared per-app work directory can
/// re-point at a different backend key when switching stages (eg `dev` ->
/// `prod`), which each own an independent remote state and so need no migration.
pub async fn init(dir: &AbsPathBuf) -> Result {
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(["init", "-reconfigure"])
		.run_async()
		.await?;
	Ok(())
}

/// Validates the opentofu file, ie the `main.tf.json`. Never needs `-var`:
/// validation is static and does not evaluate resource or encryption values.
pub async fn validate(dir: &AbsPathBuf) -> Result<String> {
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(["validate", "-json"])
		.run_async_stdout()
		.await
}

/// Show execution plan. `vars` carries anything required to read existing
/// state, eg a [`StateEncryption`] passphrase.
pub async fn plan(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["plan".into()];
	args.extend(var_args(vars));
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}

/// Apply the execution plan. `vars` carries anything required to read/write
/// state, eg a [`StateEncryption`] passphrase.
pub async fn apply(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	apply_with_vars(dir, vars, &[]).await
}

/// Apply the execution plan with Terraform variables, narrowed to `targets`
/// (resource addresses) and their dependencies when non-empty.
pub async fn apply_with_vars(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
	targets: &[String],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["apply".into(), "-auto-approve".into()];
	args.extend(var_args(vars));
	// tofu pulls in each target's dependencies but never its dependents, so a
	// targeted apply converges exactly these resources and leaves the rest of the
	// stack (notably the service roll) for the apply that follows.
	for target in targets {
		args.push(format!("-target={target}").into());
	}
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}

/// Show the current state. `vars` carries anything required to read it, eg a
/// [`StateEncryption`] passphrase.
pub async fn show(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["show".into()];
	args.extend(var_args(vars));
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}

/// Read a specific output value from the tofu state. `vars` carries anything
/// required to read it, eg a [`StateEncryption`] passphrase.
pub async fn output(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
	name: &str,
) -> Result<String> {
	let mut args: Vec<SmolStr> =
		vec!["output".into(), "-raw".into(), name.into()];
	args.extend(var_args(vars));
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
		.map(|val| val.trim().to_string())
}

/// List all resources in the state. `vars` carries anything required to read
/// it, eg a [`StateEncryption`] passphrase.
pub async fn list(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["state".into(), "list".into()];
	args.extend(var_args(vars));
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}

/// Remove a resource from the state. `vars` carries anything required to
/// read/write it, eg a [`StateEncryption`] passphrase.
pub async fn remove(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
	resource: &str,
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["state".into(), "rm".into()];
	args.extend(var_args(vars));
	args.push(resource.into());
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}

/// Destroy infrastructure. `vars` carries anything required to read/write
/// state, eg a [`StateEncryption`] passphrase.
pub async fn destroy(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["destroy".into(), "-auto-approve".into()];
	args.extend(var_args(vars));
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}

/// Destroy infrastructure, bypassing any stale state locks.
/// Used only by `force_destroy` recovery paths where we know
/// no concurrent operation is active.
pub async fn destroy_force(
	dir: &AbsPathBuf,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec![
		"destroy".into(),
		"-auto-approve".into(),
		"-lock=false".into(),
	];
	args.extend(var_args(vars));
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(args)
		.run_async_stdout()
		.await
}
