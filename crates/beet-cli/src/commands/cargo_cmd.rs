use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

/// Verbatim clone of cargo run args
#[derive(Debug, Clone, Parser)]
#[command(name = "cargo-cmd")]
pub struct CargoCmd {
	/// Error format
	#[arg(long, default_value = "build")]
	pub cargo_cmd: String,
	#[arg(long)]
	pub message_format: Option<String>,
	/// Use verbose output (-vv very verbose/build.rs output)
	#[arg(short = 'v', long, action = clap::ArgAction::Count)]
	pub verbose: u8,
	/// Do not print cargo log messages
	#[arg(short, long)]
	pub quiet: bool,
	/// Coloring: auto, always, never
	#[arg(long)]
	pub color: Option<String>,
	/// Override a configuration value
	pub config: Option<String>,
	/// Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details
	#[arg(short = 'Z', long)]
	pub z: Option<String>,
	/// Package with the target to run
	#[arg(short = 'p', long = "package")]
	pub package: Option<String>,
	/// Name of the bin target to run
	#[arg(long)]
	pub bin: Option<String>,
	/// Name of the example target to run
	#[arg(long)]
	pub example: Option<String>,
	/// Space or comma separated list of features to activate
	#[arg(short = 'F', long = "features")]
	pub features: Option<String>,
	/// Activate all available features
	#[arg(long)]
	pub all_features: bool,
	/// Do not activate the `default` feature
	#[arg(long)]
	pub no_default_features: bool,
	/// Number of parallel jobs, defaults to # of CPUs.
	#[arg(short = 'j', long)]
	pub jobs: Option<String>,
	/// Do not abort the build as soon as there is an error
	#[arg(long)]
	pub keep_going: bool,
	/// Build artifacts in release mode, with optimizations
	#[arg(long)]
	pub release: bool,
	/// Build artifacts with the specified profile
	#[arg(long)]
	pub profile: Option<String>,
	/// Build for the target triple
	#[arg(long)]
	pub target: Option<String>,
	/// Directory for all generated artifacts
	#[arg(long)]
	pub target_dir: Option<String>,
	/// Output build graph in JSON (unstable)
	#[arg(long)]
	pub unit_graph: bool,
	/// Timing output formats (unstable) (comma separated): html, json
	#[arg(long)]
	pub timings: Option<String>,
	/// Path to Cargo.toml
	#[arg(long)]
	pub manifest_path: Option<String>,
	/// Path to Cargo.lock (unstable)
	#[arg(long)]
	pub lockfile_path: Option<String>,
	/// Ignore `rust-version` specification in packages
	#[arg(long)]
	pub ignore_rust_version: bool,
	/// Assert that `Cargo.lock` will remain unchanged
	#[arg(long)]
	pub locked: bool,
	/// Run without accessing the network
	#[arg(long)]
	pub offline: bool,
	/// Equivalent to specifying both --locked and --offline
	#[arg(long)]
	pub frozen: bool,
	/// Any additional arguments passed to cargo
	#[arg(trailing_var_arg = true)]
	pub args: Vec<String>,
}

impl Default for CargoCmd {
	fn default() -> Self { Self::parse_from(&[""]) }
}

impl CargoCmd {
	/// Best effort attempt to retrieve the path to the executable.
	/// In the case of a wasm target, the path will have a `.wasm` extension.
	pub fn exe_path(&self) -> PathBuf {
		let target_dir = std::env::var("CARGO_TARGET_DIR")
			.unwrap_or_else(|_| "target".to_string());
		let mut path = PathBuf::from(target_dir);

		if let Some(target) = &self.target {
			path.push(target);
		}

		if self.release {
			path.push("release");
		} else {
			path.push("debug");
		}

		if let Some(example) = &self.example {
			path.push("examples");
			path.push(example);
		// package examples are not nested under package name
		} else if let Some(pkg) = &self.package {
			path.push(pkg);
		}
		if let Some(bin) = &self.bin {
			path.push(bin);
		}

		if let Some("wasm32-unknown-unknown") = self.target.as_deref() {
			path.set_extension("wasm");
		}

		path
	}

	pub fn spawn(&self) -> Result<()> {
		let CargoCmd {
			cargo_cmd,
			args,
			message_format,
			verbose,
			quiet,
			color,
			config,
			z,
			package,
			bin,
			example,
			features,
			all_features,
			no_default_features,
			jobs,
			keep_going,
			release,
			profile,
			target,
			target_dir,
			unit_graph,
			timings,
			manifest_path,
			lockfile_path,
			ignore_rust_version,
			locked,
			offline,
			frozen,
		} = self;
		let mut cmd = Command::new("cargo");
		cmd.arg(cargo_cmd);


		if let Some(format) = message_format {
			cmd.arg("--message-format").arg(format);
		}

		for _ in 0..*verbose {
			cmd.arg("-v");
		}

		if *quiet {
			cmd.arg("--quiet");
		}

		if let Some(c) = color {
			cmd.arg("--color").arg(c);
		}

		if let Some(cfg) = config {
			cmd.arg("--config").arg(cfg);
		}

		if let Some(z_flag) = z {
			cmd.arg("-Z").arg(z_flag);
		}

		if let Some(pkg) = package {
			cmd.arg("--package").arg(pkg);
		}

		if let Some(b) = bin {
			cmd.arg("--bin").arg(b);
		}

		if let Some(ex) = example {
			cmd.arg("--example").arg(ex);
		}

		if let Some(feat) = features {
			cmd.arg("--features").arg(feat);
		}

		if *all_features {
			cmd.arg("--all-features");
		}

		if *no_default_features {
			cmd.arg("--no-default-features");
		}

		if let Some(j) = jobs {
			cmd.arg("-j").arg(j);
		}

		if *keep_going {
			cmd.arg("--keep-going");
		}

		if *release {
			cmd.arg("--release");
		}

		if let Some(prof) = profile {
			cmd.arg("--profile").arg(prof);
		}

		if let Some(tgt) = target {
			cmd.arg("--target").arg(tgt);
		}

		if let Some(dir) = target_dir {
			cmd.arg("--target-dir").arg(dir);
		}

		if *unit_graph {
			cmd.arg("--unit-graph");
		}

		if let Some(tim) = timings {
			cmd.arg("--timings").arg(tim);
		}

		if let Some(path) = manifest_path {
			cmd.arg("--manifest-path").arg(path);
		}

		if let Some(lock) = lockfile_path {
			cmd.arg("--lockfile-path").arg(lock);
		}

		if *ignore_rust_version {
			cmd.arg("--ignore-rust-version");
		}

		if *locked {
			cmd.arg("--locked");
		}

		if *offline {
			cmd.arg("--offline");
		}

		if *frozen {
			cmd.arg("--frozen");
		}

		cmd.args(args);

		cmd.status()?.exit_ok()?;
		Ok(())
	}
}

//cargo build -p beet_site --message-format=json | jq -r 'select(.reason == "compiler-artifact" and .target.kind == ["bin"]) | .filenames[]'
