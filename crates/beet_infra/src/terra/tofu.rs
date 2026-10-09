//! Module for interacting with tofu
//!
//! ## Architecture
//!
//! The default approach is a single state backend, ie a directory or s3 bucket,
//! with each stack (app-stage pair) having its own state under a flat key,
//! ie `beet--dev--tofu-tfstate` in the bucket [`S3Backend`] names.
//!
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Irreversibly remove the backend, destroying the tofu state for **all applications**.
pub async fn dangerously_destroy_backend(backend: &ResolvedBackend) -> Result {
	match backend {
		ResolvedBackend::Local(local) => {
			fs_ext::remove_async(local.path()).await?;
		}
		ResolvedBackend::S3(_) => {
			backend.store()?.store_remove().await?;
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

/// Where downloaded provider plugins are shared between work directories.
///
/// Every `init` runs in a work directory of its own (a deploy's, a test's), so
/// without a shared cache each one re-downloads the full provider set: the AWS
/// provider alone is most of a gigabyte, and the concurrent `validate` tests
/// each pay it. Honours an explicitly-set `TF_PLUGIN_CACHE_DIR`, else picks the
/// conventional one and creates it, since tofu will not create it itself and
/// silently skips caching when it is missing.
fn plugin_cache_dir() -> Option<AbsPath> {
	let dir = match env_ext::var("TF_PLUGIN_CACHE_DIR") {
		Ok(dir) if !dir.is_empty() => AbsPath::new(dir.as_str()).ok()?,
		_ => AbsPath::new(env_ext::var("HOME").ok()?.as_str())
			.ok()?
			.join(".cache/tofu-plugins"),
	};
	fs_ext::create_dir_all(&dir).ok()?;
	Some(dir)
}

/// Hand `vars` to a tofu process as `TF_VAR_<key>` environment variables,
/// the channel every subcommand reads a variable from (`init` included,
/// which evaluates the encryption config). The environment rather than
/// `-var` on argv, since a stack's [`StateEncryption`] passphrase rides here
/// and argv is readable by every process on the machine; that one value is
/// also redacted from the process's reported output. Nothing is ever
/// written into `main.tf.json`.
fn with_vars(
	process: ChildProcess,
	vars: &[(SmolStr, SmolStr)],
) -> ChildProcess {
	vars.iter().fold(process, |process, (key, value)| {
		let process = match key.as_str() == STATE_ENCRYPTION_VAR {
			true => process.with_secret(value.clone()),
			false => process,
		};
		process.with_env(format!("TF_VAR_{key}"), value.clone())
	})
}

/// Export the provider schema based on `./providers.tf.json`
pub async fn export_schema(dir: &AbsPath) -> Result<String> {
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(["providers", "schema", "-json"])
		.run_async_stdout()
		.await
}

/// Initialize an opentofu directory, using the `./providers.tf.json`.
/// `vars` carries what the encryption config evaluates at init, ie a
/// [`StateEncryption`] passphrase.
pub async fn init(dir: &AbsPath, vars: &[(SmolStr, SmolStr)]) -> Result {
	with_vars(init_process(dir), vars).run_async().await?;
	Ok(())
}

/// The `tofu init` invocation, without its vars.
///
/// `-reconfigure` lets the shared per-app work directory re-point at a
/// different backend key when switching stages (eg `dev` -> `prod`), which
/// each own an independent remote state and so need no migration. `-upgrade`
/// lets the lock file follow a provider bump: every provider is pinned to one
/// exact release ([`Provider::version`](super::Provider::version)), so it selects that release and
/// nothing newer, where without it a lock naming the previous release refuses
/// the init.
fn init_process(dir: &AbsPath) -> ChildProcess {
	let process = tofu_process().with_cwd(dir.clone()).with_args([
		"init",
		"-reconfigure",
		"-upgrade",
	]);
	match plugin_cache_dir() {
		Some(cache) => process.with_env("TF_PLUGIN_CACHE_DIR", cache.as_str()),
		None => process,
	}
}

/// Validates the opentofu file, ie the `main.tf.json`. Never needs `-var`:
/// validation is static and does not evaluate resource or encryption values.
pub async fn validate(dir: &AbsPath) -> Result<String> {
	tofu_process()
		.with_cwd(dir.clone())
		.with_args(["validate", "-json"])
		.run_async_stdout()
		.await
}

/// Show execution plan. `vars` carries anything required to read existing
/// state, eg a [`StateEncryption`] passphrase.
pub async fn plan(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	with_vars(
		tofu_process().with_cwd(dir.clone()).with_args(["plan"]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// The writes an apply narrowed to `targets` would make, planned against the
/// state alone (`-refresh=false`): what the declarations changed, for the cost
/// of reading the state rather than refreshing every resource, which is how an
/// apply checks what it is about to write before it writes anything.
pub async fn planned_changes(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
	targets: &[String],
) -> Result<Vec<PlannedChange>> {
	let mut args: Vec<SmolStr> = vec![
		"plan".into(),
		"-refresh=false".into(),
		"-input=false".into(),
		"-json".into(),
	];
	for target in targets {
		args.push(format!("-target={target}").into());
	}
	with_vars(tofu_process().with_cwd(dir.clone()).with_args(args), vars)
		.run_async_stdout()
		.await?
		.xmap(|output| PlannedChange::parse(&output))
		.xok()
}

/// One write a plan would make, as `tofu plan -json` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedChange {
	/// The resource address, ie `cloudflare_r2_bucket_lock.cold_backups`.
	pub address: SmolStr,
	/// Its declared type, ie `cloudflare_r2_bucket_lock`.
	pub resource_type: SmolStr,
	/// What the apply would do: `create`, `update`, `replace` or `delete`.
	pub action: SmolStr,
}

impl PlannedChange {
	/// The actions that write: a `read`, a `noop` and a `move` call the
	/// provider for no change.
	const WRITES: [&'static str; 4] = ["create", "update", "replace", "delete"];

	/// Every write in `tofu plan -json`'s output, one json message a line,
	/// from its `planned_change` messages.
	pub fn parse(output: &str) -> Vec<Self> {
		output
			.lines()
			.filter_map(|line| {
				serde_json::from_str::<serde_json::Value>(line).ok()
			})
			.filter(|message| message["type"] == "planned_change")
			.filter_map(|message| {
				let change = &message["change"];
				let resource = &change["resource"];
				Self {
					address: resource["addr"].as_str()?.into(),
					resource_type: resource["resource_type"].as_str()?.into(),
					action: change["action"].as_str()?.into(),
				}
				.xmap(Some)
			})
			.filter(|change| Self::WRITES.contains(&change.action.as_str()))
			.collect()
	}
}

/// Apply the execution plan. `vars` carries anything required to read/write
/// state, eg a [`StateEncryption`] passphrase.
pub async fn apply(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	apply_with_vars(dir, vars, &[]).await
}

/// Apply the execution plan with Terraform variables, narrowed to `targets`
/// (resource addresses) and their dependencies when non-empty.
pub async fn apply_with_vars(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
	targets: &[String],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["apply".into(), "-auto-approve".into()];
	// tofu pulls in each target's dependencies but never its dependents, so a
	// targeted apply converges exactly these resources and leaves the rest of the
	// stack (notably the service roll) for the apply that follows.
	for target in targets {
		args.push(format!("-target={target}").into());
	}
	with_vars(tofu_process().with_cwd(dir.clone()).with_args(args), vars)
		.run_async_stdout()
		.await
}

/// Apply with every resource in `replaces` (addresses) forced to be
/// destroyed and recreated: how a terraform-derived secret rolls, since
/// the same apply re-parks what the new resource derives.
pub async fn apply_replacing(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
	replaces: &[String],
) -> Result<String> {
	let mut args: Vec<SmolStr> = vec!["apply".into(), "-auto-approve".into()];
	for resource in replaces {
		args.push(format!("-replace={resource}").into());
	}
	with_vars(tofu_process().with_cwd(dir.clone()).with_args(args), vars)
		.run_async_stdout()
		.await
}

/// Show the current state. `vars` carries anything required to read it, eg a
/// [`StateEncryption`] passphrase.
pub async fn show(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	with_vars(
		tofu_process().with_cwd(dir.clone()).with_args(["show"]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// Read a specific output value from the tofu state. `vars` carries anything
/// required to read it, eg a [`StateEncryption`] passphrase.
pub async fn output(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
	name: &str,
) -> Result<String> {
	with_vars(
		tofu_process()
			.with_cwd(dir.clone())
			.with_args(["output", "-raw", name]),
		vars,
	)
	.run_async_stdout()
	.await
	.map(|val| val.trim().to_string())
}

/// List all resources in the state. `vars` carries anything required to read
/// it, eg a [`StateEncryption`] passphrase.
pub async fn list(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	with_vars(
		tofu_process()
			.with_cwd(dir.clone())
			.with_args(["state", "list"]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// The state as JSON, the input to a rewrite. `vars` carries anything
/// required to read it, eg a [`StateEncryption`] passphrase.
pub async fn state_pull(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	with_vars(
		tofu_process()
			.with_cwd(dir.clone())
			.with_args(["state", "pull"]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// Write `file` as the state, replacing what the backend holds. Tofu reads
/// the old state first and refuses a lineage change or a serial behind it,
/// and skips the write when nothing changed, so a caller rewriting the state
/// it pulled bumps the serial.
///
/// `vars` carries anything required to read the old and write the new, eg a
/// [`StateEncryption`] passphrase; across a [`StateCrossing`] the read and the
/// write differ by method, never by what the destination needs, which is what
/// keeps the push's own refresh of the destination readable.
pub async fn state_push(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
	file: &AbsPath,
) -> Result<String> {
	with_vars(
		tofu_process().with_cwd(dir.clone()).with_args([
			"state",
			"push",
			file.as_str(),
		]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// Remove a resource from the state. `vars` carries anything required to
/// read/write it, eg a [`StateEncryption`] passphrase.
pub async fn remove(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
	resource: &str,
) -> Result<String> {
	with_vars(
		tofu_process()
			.with_cwd(dir.clone())
			.with_args(["state", "rm", resource]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// Destroy infrastructure. `vars` carries anything required to read/write
/// state, eg a [`StateEncryption`] passphrase.
pub async fn destroy(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	with_vars(
		tofu_process()
			.with_cwd(dir.clone())
			.with_args(["destroy", "-auto-approve"]),
		vars,
	)
	.run_async_stdout()
	.await
}

/// Destroy infrastructure, bypassing any stale state locks.
/// Used only by the `force` recovery path (`Project::tofu_destroy`), where we
/// know no concurrent operation is active.
pub async fn destroy_force(
	dir: &AbsPath,
	vars: &[(SmolStr, SmolStr)],
) -> Result<String> {
	with_vars(
		tofu_process().with_cwd(dir.clone()).with_args([
			"destroy",
			"-auto-approve",
			"-lock=false",
		]),
		vars,
	)
	.run_async_stdout()
	.await
}

#[cfg(test)]
mod test {
	use super::*;

	/// A provider bump must reach a work directory whose lock file names the
	/// previous release, which only `-upgrade` allows.
	#[beet_core::test]
	fn init_lets_the_lock_follow_the_pin() {
		init_process(&AbsPath::new_unchecked("/tmp/stack"))
			.to_string()
			.xpect_contains("init -reconfigure -upgrade");
	}

	/// A plan's writes are read off its `planned_change` messages, and a read,
	/// a no-op and every other message are not writes.
	#[beet_core::test]
	fn reads_the_planned_writes() {
		let change = |addr: &str, kind: &str, action: &str| {
			serde_json::json!({
				"type": "planned_change",
				"change": {
					"resource": { "addr": addr, "resource_type": kind },
					"action": action,
				},
			})
			.to_string()
		};
		let output = [
			r#"{"type":"version","tofu":"1.10.0"}"#.to_string(),
			change(
				"cloudflare_r2_bucket_lock.cold",
				"cloudflare_r2_bucket_lock",
				"update",
			),
			change("aws_instance.box", "aws_instance", "replace"),
			change("data.aws_ami.debian", "aws_ami", "read"),
			r#"{"type":"change_summary","changes":{"add":0}}"#.to_string(),
		]
		.join("\n");
		PlannedChange::parse(&output).xpect_eq(vec![
			PlannedChange {
				address: "cloudflare_r2_bucket_lock.cold".into(),
				resource_type: "cloudflare_r2_bucket_lock".into(),
				action: "update".into(),
			},
			PlannedChange {
				address: "aws_instance.box".into(),
				resource_type: "aws_instance".into(),
				action: "replace".into(),
			},
		]);
	}
}
