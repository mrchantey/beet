//! Module for interacting with tofu
//!
//! ## Architecture
//!
//! The default approach is a single state backend, ie a directory or s3 bucket,
//! with each stack (app-stage pair) having its own state under a flat key,
//! ie `beet--dev--tofu-tfstate` in the bucket [`S3Backend`] names.
//!
use crate::prelude::*;
use crate::terra::TofuVar;
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
/// `-var` on argv, since argv is readable by every process on the machine;
/// a [`secret`](TofuVar::secret) value is also redacted from the process's
/// reported output. Nothing is ever written into `main.tf.json`.
fn with_vars(process: ChildProcess, vars: &[TofuVar]) -> ChildProcess {
	vars.iter().fold(process, |process, var| {
		let process = match var.secret {
			true => process.with_secret(var.value.clone()),
			false => process,
		};
		process.with_env(format!("TF_VAR_{}", var.key), var.value.clone())
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
pub async fn init(dir: &AbsPath, vars: &[TofuVar]) -> Result {
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
pub async fn plan(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
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
	vars: &[TofuVar],
	targets: &[String],
) -> Result<Vec<PlannedChange>> {
	assess(
		dir,
		vars,
		targets.iter().map(|target| format!("-target={target}")),
	)
	.await
}

/// The writes a [`destroy`] would make, planned the same way with
/// `-destroy`: a `delete` for every resource the state holds, whatever the
/// declarations say, which is the change set a teardown is judged on.
pub async fn planned_destroy(
	dir: &AbsPath,
	vars: &[TofuVar],
) -> Result<Vec<PlannedChange>> {
	assess(dir, vars, ["-destroy".to_string()]).await
}

/// `tofu plan -json` against the state alone with `args` appended, read for
/// its writes. An assessment writes nothing, state included, so it takes no
/// lock: a lock an interrupted run left behind never blocks the
/// `destroy --force` that exists to clear it.
async fn assess(
	dir: &AbsPath,
	vars: &[TofuVar],
	args: impl IntoIterator<Item = String>,
) -> Result<Vec<PlannedChange>> {
	[
		"plan",
		"-refresh=false",
		"-lock=false",
		"-input=false",
		"-json",
	]
	.into_iter()
	.map(SmolStr::from)
	.chain(args.into_iter().map(SmolStr::from))
	.collect::<Vec<_>>()
	.xmap(|args| tofu_process().with_cwd(dir.clone()).with_args(args))
	.xmap(|process| with_vars(process, vars))
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
pub async fn apply(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
	apply_with_vars(dir, vars, &[]).await
}

/// Apply the execution plan with Terraform variables, narrowed to `targets`
/// (resource addresses) and their dependencies when non-empty.
pub async fn apply_with_vars(
	dir: &AbsPath,
	vars: &[TofuVar],
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
	vars: &[TofuVar],
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
pub async fn show(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
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
	vars: &[TofuVar],
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
pub async fn list(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
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
pub async fn state_pull(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
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
	vars: &[TofuVar],
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
	vars: &[TofuVar],
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
pub async fn destroy(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
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
pub async fn destroy_force(dir: &AbsPath, vars: &[TofuVar]) -> Result<String> {
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

/// Support for the tests that drive the real cli. They run in the normal
/// suite and fail rather than skip without tofu, which is part of the
/// machine's provisioning, never optional.
#[cfg(test)]
pub(crate) mod test_support {
	use super::*;

	/// What a test answers a missing tofu with: the recipe that installs it.
	const NOT_FOUND: &str = "this test drives the tofu cli, which is not on \
		PATH: install it with the machine provisioning's `just install-apps` \
		(its `opentofu` package, arch-config's justfile), or see \
		https://opentofu.org/docs/intro/install";

	/// tofu, failing naming [`NOT_FOUND`]'s recipe when it is absent.
	fn process() -> ChildProcess {
		ChildProcess::new("tofu").with_not_found(NOT_FOUND)
	}

	/// Fail naming the recipe that installs tofu unless it is on PATH, for a
	/// test reaching it through a [`Project`](crate::terra::Project).
	pub(crate) async fn require() -> Result {
		process().with_args(["version"]).run_async().await?;
		Ok(())
	}

	/// `tofu init` in `dir` with no network: every provider its
	/// `main.tf.json` requires is read from the shared
	/// [plugin cache](plugin_cache_dir) as a filesystem mirror, and a registry
	/// is never asked, so a test is hermetic. `vars` as [`init`] takes them.
	/// Refuses naming a pinned provider the cache lacks, which any networked
	/// init (ie `just beet-validate`) fills.
	pub(crate) async fn init_offline(
		dir: &AbsPath,
		vars: &[TofuVar],
	) -> Result {
		let cache = plugin_cache_dir()
			.ok_or_else(|| bevyhow!("no tofu plugin cache: `HOME` is unset"))?;
		let config: serde_json::Value = serde_json::from_str(
			&fs_ext::read_to_string(dir.join("main.tf.json"))?,
		)?;
		let required = config["terraform"]["required_providers"]
			.as_object()
			.cloned()
			.unwrap_or_default();
		for provider in required.values() {
			let source = provider["source"].as_str().unwrap_or_default();
			let version = provider["version"]
				.as_str()
				.unwrap_or_default()
				.trim_start_matches('=')
				.trim();
			let release = cache
				.join("registry.opentofu.org")
				.join(source)
				.join(version);
			if !fs_ext::exists(&release)? {
				bevybail!(
					"`{source}` {version} is not in the tofu plugin cache \
					`{cache}`, and a test never downloads it: fill the cache \
					with one networked init, ie `just beet-validate`"
				);
			}
		}
		with_vars(
			process().with_cwd(dir.clone()).with_args([
				"init".to_string(),
				"-input=false".to_string(),
				format!("-plugin-dir={cache}"),
			]),
			vars,
		)
		.run_async()
		.await?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A destroy is assessed on its own change set: an applied resource that
	/// the declarations still hold plans no write, and its destroy plans its
	/// deletion. Builtin resources alone, so no provider is ever fetched.
	#[beet_core::test(timeout_ms = 60_000)]
	async fn plans_the_destroy_it_assesses() {
		let dir = TempDir::new().unwrap();
		fs_ext::write(
			dir.join("main.tf.json"),
			serde_json::json!({
				"resource": { "terraform_data": { "box": { "input": "box" } } },
			})
			.to_string(),
		)
		.unwrap();
		test_support::init_offline(&dir, &[]).await.unwrap();
		apply(&dir, &[]).await.unwrap();
		planned_changes(&dir, &[], &[]).await.unwrap().xpect_empty();
		planned_destroy(&dir, &[]).await.unwrap().xpect_eq(vec![
			PlannedChange {
				address: "terraform_data.box".into(),
				resource_type: "terraform_data".into(),
				action: "delete".into(),
			},
		]);
	}

	/// A secret variable rides the child's environment and is redacted from
	/// what the invocation reports, a plain one only rides it.
	#[beet_core::test]
	fn redacts_a_secret_variable() {
		let process = with_vars(
			tofu_process().with_args(["apply", "write-only-canary", "dev"]),
			&[
				TofuVar::secret("host_key", "write-only-canary"),
				TofuVar::new("stage", "dev"),
			],
		);
		process
			.to_string()
			.xpect_contains(ChildProcess::REDACTED)
			.xpect_contains("dev")
			.xnot()
			.xpect_contains("write-only-canary");
		format!("{:?}", TofuVar::secret("host_key", "write-only-canary"))
			.xnot()
			.xpect_contains("write-only-canary");
	}

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
