//! Async AWS CLI helpers for S3 using async-process.
//!
//! This module provides a small, well-documented surface for invoking the AWS
//! CLI (specifically aws s3 sync) from async Rust code without pulling in the
//! full AWS SDK. It is intended for CI, build, and tooling flows where the AWS
//! CLI is already installed and configured (credentials, profiles, etc).
//!
//! Key points
//! - Uses async-process for non-blocking process execution.
//! - Uses AbsPathBuf for local filesystem paths in helpers and never shells out.
//! - Configurable via S3Sync, which holds an AwsCli and sync flags.
//! - Preserves include/exclude ordering (important for AWS CLI filter evaluation).
//!
//! We do not validate that the AWS CLI is installed; failures to spawn will
//! surface naturally as process errors.
use async_process::Command;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::process::Stdio;

/// A minimal AWS CLI driver with a few global settings (profile, region).
///
/// This type helps constructing argument vectors for commands and running them
/// via async-process. The driver prefers passing --profile and --region flags
/// over mutating environment, keeping behavior explicit in the command.
#[derive(Debug, Default, Clone)]
pub struct AwsCli {
	/// Optional `--profile` value.
	pub profile: Option<String>,
	/// Optional `--region` value.
	pub region: Option<String>,
	/// Pass `--no-sign-request` for public buckets/use-cases.
	pub no_sign_request: bool,
	/// Any additional global args to place in front of subcommands.
	pub extra_global_args: Vec<String>,
}

impl AwsCli {
	/// Create an empty driver (no profile/region).
	pub fn new() -> Self { Self::default() }

	/// Set `--profile`.
	pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
		self.profile = Some(profile.into());
		self
	}

	/// Set `--region`.
	pub fn with_region(mut self, region: impl Into<String>) -> Self {
		self.region = Some(region.into());
		self
	}

	/// Enable/disable `--no-sign-request`.
	pub fn with_no_sign_request(mut self, enabled: bool) -> Self {
		self.no_sign_request = enabled;
		self
	}

	/// Append a global arg that will come right after `aws`.
	pub fn with_global_arg(mut self, arg: impl Into<String>) -> Self {
		self.extra_global_args.push(arg.into());
		self
	}

	/// Build an argv for `aws s3 sync <src> <dst> ...`.
	///
	/// Exposed for testability and custom flows; typical users will call
	/// `S3Sync::send()` which uses this under the hood.
	pub fn build_s3_sync_args(
		&self,
		src: &str,
		dst: &str,
		opts: &S3Sync,
	) -> Vec<String> {
		let mut argv = vec!["aws".to_string()];
		if let Some(profile) = &self.profile {
			argv.push("--profile".into());
			argv.push(profile.clone());
		}
		if let Some(region) = &self.region {
			argv.push("--region".into());
			argv.push(region.clone());
		}
		if self.no_sign_request {
			argv.push("--no-sign-request".into());
		}
		argv.extend(self.extra_global_args.iter().cloned());

		argv.push("s3".into());
		argv.push("sync".into());
		argv.push(src.into());
		argv.push(dst.into());
		argv.extend(opts.to_args());
		argv
	}

	/// Run a previously constructed argv (first element must be the program).
	async fn run_argv(&self, argv: Vec<String>) -> Result {
		let (prog, rest) =
			argv.split_first().ok_or_else(|| bevyhow!("empty argv"))?;
		let status = Command::new(prog)
			.args(rest)
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.status()
			.await?;

		if !status.success() {
			bevybail!("aws cli exited with non-zero status: {:?}", status);
		}
		Ok(())
	}
}

/// Represents a single include/exclude directive. Order matters for the AWS CLI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Filter {
	Exclude(String),
	Include(String),
}

impl S3Filter {
	fn to_args(&self) -> [String; 2] {
		match self {
			S3Filter::Exclude(p) => ["--exclude".into(), p.clone()],
			S3Filter::Include(p) => ["--include".into(), p.clone()],
		}
	}
}

/// Direction of the sync to drive URI validation in send().
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum S3Direction {
	Push, // local -> s3
	Pull, // s3 -> local
}

/// Builder for `aws s3 sync`, including CLI configuration, endpoints, and flags.
///
/// This struct preserves the order of include/exclude rules, as the AWS CLI
/// evaluates them in the order provided. Other booleans map directly to known
/// flags. Anything else can be passed via `additional_args`.
///
/// Required fields
/// - `cli`: the AwsCli configuration
/// - `src`, `dst`: the sync endpoints; helpers `push(...)` and `pull(...)` construct these.
///
/// Mapping
/// - `delete`            -> `--delete`
/// - `size_only`         -> `--size-only`
/// - `dry_run`           -> `--dryrun`
/// - `no_progress`       -> `--no-progress`
/// - `exact_timestamps`  -> `--exact-timestamps`
/// - `follow_symlinks`   -> `--follow-symlinks`
/// - `acl`               -> `--acl <value>` (e.g. `public-read`)
/// - `filters`           -> `--exclude <pat>` / `--include <pat>` in provided order
/// - `additional_args`   -> appended verbatim at the end
#[derive(Debug, Clone)]
pub struct S3Sync {
	pub cli: AwsCli,
	pub src: String,
	pub dst: String,
	dir: S3Direction,
	// flags
	pub delete: bool,
	pub size_only: bool,
	pub dry_run: bool,
	pub no_progress: bool,
	pub exact_timestamps: bool,
	pub follow_symlinks: bool,
	pub acl: Option<String>,
	pub filters: Vec<S3Filter>,
	pub additional_args: Vec<String>,
}

impl Default for S3Sync {
	fn default() -> Self {
		Self {
			cli: default(),
			src: String::new(),
			dst: String::new(),
			dir: S3Direction::Push,
			delete: false,
			size_only: false,
			dry_run: false,
			no_progress: true, // prefer quiet output by default
			exact_timestamps: false,
			follow_symlinks: false,
			acl: None,
			filters: Vec::new(),
			additional_args: Vec::new(),
		}
	}
}

impl S3Sync {
	/// Construct a push sync (local -> s3) from AbsPathBuf to S3 URI.
	///
	/// Local path must be absolute (AbsPathBuf ensures that). URI is validated during send.
	pub fn push(
		cli: AwsCli,
		local_dir: AbsPathBuf,
		s3_uri: impl AsRef<str>,
	) -> Self {
		Self {
			cli,
			src: local_dir.to_string_lossy().to_string(),
			dst: s3_uri.as_ref().to_string(),
			dir: S3Direction::Push,
			..Default::default()
		}
	}

	/// Construct a pull sync (s3 -> local) from S3 URI to AbsPathBuf.
	///
	/// Local path must be absolute (AbsPathBuf ensures that). URI is validated during send.
	pub fn pull(
		cli: AwsCli,
		s3_uri: impl AsRef<str>,
		local_dir: AbsPathBuf,
	) -> Self {
		Self {
			cli,
			src: s3_uri.as_ref().to_string(),
			dst: local_dir.to_string_lossy().to_string(),
			dir: S3Direction::Pull,
			..Default::default()
		}
	}

	/// Execute the configured sync.
	///
	/// For push, validates that `dst` is an S3 URI. For pull, validates that `src` is an S3 URI.
	pub async fn send(&self) -> Result {
		match self.dir {
			S3Direction::Push => {
				if !is_s3_uri(&self.dst) {
					bevybail!("expected S3 URI (s3://...), got: {}", self.dst);
				}
			}
			S3Direction::Pull => {
				if !is_s3_uri(&self.src) {
					bevybail!("expected S3 URI (s3://...), got: {}", self.src);
				}
			}
		}
		let argv = self.cli.build_s3_sync_args(&self.src, &self.dst, self);
		self.cli.run_argv(argv).await
	}

	pub fn delete(mut self, value: bool) -> Self {
		self.delete = value;
		self
	}
	pub fn size_only(mut self, value: bool) -> Self {
		self.size_only = value;
		self
	}
	pub fn dry_run(mut self, value: bool) -> Self {
		self.dry_run = value;
		self
	}
	pub fn no_progress(mut self, value: bool) -> Self {
		self.no_progress = value;
		self
	}
	pub fn exact_timestamps(mut self, value: bool) -> Self {
		self.exact_timestamps = value;
		self
	}
	pub fn follow_symlinks(mut self, value: bool) -> Self {
		self.follow_symlinks = value;
		self
	}
	/// Shorthand to set `--acl public-read`.
	pub fn acl_public_read(mut self) -> Self {
		self.acl = Some("public-read".to_string());
		self
	}
	/// Set a specific ACL string (e.g. `public-read`, `bucket-owner-full-control`).
	pub fn acl(mut self, value: impl Into<String>) -> Self {
		self.acl = Some(value.into());
		self
	}
	/// Append an exclude rule. Preserves order relative to includes.
	pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
		self.filters.push(S3Filter::Exclude(pattern.into()));
		self
	}
	/// Append an include rule. Preserves order relative to excludes.
	pub fn include(mut self, pattern: impl Into<String>) -> Self {
		self.filters.push(S3Filter::Include(pattern.into()));
		self
	}
	/// Append a raw argument verbatim to the end of the argv.
	pub fn arg(mut self, arg: impl Into<String>) -> Self {
		self.additional_args.push(arg.into());
		self
	}

	/// Convert to argv flags in a deterministic order.
	fn to_args(&self) -> Vec<String> {
		let mut out = Vec::new();
		if self.delete {
			out.push("--delete".into());
		}
		if self.size_only {
			out.push("--size-only".into());
		}
		if self.dry_run {
			out.push("--dryrun".into());
		}
		if self.no_progress {
			out.push("--no-progress".into());
		}
		if self.exact_timestamps {
			out.push("--exact-timestamps".into());
		}
		if self.follow_symlinks {
			out.push("--follow-symlinks".into());
		}
		if let Some(acl) = &self.acl {
			out.push("--acl".into());
			out.push(acl.clone());
		}
		for f in &self.filters {
			let [flag, val] = f.to_args();
			out.push(flag);
			out.push(val);
		}
		out.extend(self.additional_args.iter().cloned());
		out
	}
}

fn is_s3_uri(s: &str) -> bool { s.starts_with("s3://") }

#[cfg(test)]
mod test {
	use super::*;

	use sweet::prelude::*;

	#[sweet::test]
	fn builds_basic_sync_args() {
		let aws = AwsCli::new()
			.with_profile("dev")
			.with_region("us-west-2")
			.with_no_sign_request(true);

		let local = AbsPathBuf::new("foo/bar").unwrap();

		let opts = S3Sync::default()
			.delete(true)
			.size_only(true)
			.dry_run(true)
			.no_progress(true)
			.exact_timestamps(true)
			.follow_symlinks(true)
			.acl_public_read()
			.include("public/**")
			.exclude("node_modules/**")
			.arg("--storage-class")
			.arg("STANDARD_IA");

		let argv = aws.build_s3_sync_args(
			&local.to_string_lossy(),
			"s3://my-bucket/site",
			&opts,
		);

		// Program and globals
		argv[0].xpect_eq("aws");
		argv.contains(&"--profile".to_string()).xpect_true();
		argv.contains(&"dev".to_string()).xpect_true();
		argv.contains(&"--region".to_string()).xpect_true();
		argv.contains(&"us-west-2".to_string()).xpect_true();
		argv.contains(&"--no-sign-request".to_string()).xpect_true();

		// Options
		let flags = [
			"--delete",
			"--size-only",
			"--dryrun",
			"--no-progress",
			"--exact-timestamps",
			"--follow-symlinks",
			"--acl",
		];
		for f in flags {
			argv.contains(&f.to_string()).xpect_true();
		}
		argv.contains(&"public-read".to_string()).xpect_true();
		argv.contains(&"--storage-class".to_string()).xpect_true();
		argv.contains(&"STANDARD_IA".to_string()).xpect_true();
	}

	#[sweet::test]
	fn preserves_filter_order() {
		let aws = AwsCli::new();
		let local = AbsPathBuf::new("some/dir").unwrap();

		let opts = S3Sync::default()
			.exclude("node_modules/**")
			.include("public/**")
			.exclude("**/*.map")
			.include("assets/**");

		let argv = aws.build_s3_sync_args(
			&local.to_string_lossy(),
			"s3://bucket/prefix",
			&opts,
		);

		// Extract only the filter flags/patterns in order
		let mut filter_pairs = Vec::<(String, String)>::new();
		let mut i = 0usize;
		while i + 1 < argv.len() {
			match argv[i].as_str() {
				"--exclude" | "--include" => {
					filter_pairs.push((argv[i].clone(), argv[i + 1].clone()));
					i += 2;
				}
				_ => i += 1,
			}
		}

		filter_pairs.xpect_eq(vec![
			("--exclude".into(), "node_modules/**".into()),
			("--include".into(), "public/**".into()),
			("--exclude".into(), "**/*.map".into()),
			("--include".into(), "assets/**".into()),
		]);
	}

	#[sweet::test]
	async fn rejects_non_s3_uri() {
		let local = AbsPathBuf::new("out").unwrap();
		let sync = S3Sync::push(AwsCli::new(), local, "not-an-s3-uri");
		let err = sync.send().await.unwrap_err();
		err.to_string().contains("expected S3 URI").xpect_true();
	}
}
