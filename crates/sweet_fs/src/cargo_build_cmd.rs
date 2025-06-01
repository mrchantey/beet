use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

/// Verbatim clone of cargo build/run args
#[derive(Debug, Clone, Parser)]
#[cfg_attr(feature = "bevy", derive(bevy::prelude::Resource))]
pub struct CargoBuildCmd {
	/// The top level command to run: `build`, `run`, `test`, etc.
	#[arg(long, default_value = "build")]
	pub cmd: String,
	/// Package with the target to run
	#[arg(short = 'p', long = "package")]
	pub package: Option<String>,
	/// Name of the bin target to run
	#[arg(long)]
	pub bin: Option<String>,
	/// Name of the example target to run
	#[arg(long)]
	pub example: Option<String>,
	/// Specify the integration test to
	#[arg(long)]
	pub test: Option<String>,
	/// Build artifacts in release mode, with optimizations
	#[arg(long)]
	pub release: bool,
	/// Only test lib
	#[arg(long)]
	pub lib: bool,
	/// Only test docs
	#[arg(long)]
	pub doc: bool,
	/// Build for the target triple
	#[arg(long)]
	pub target: Option<String>,
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
	#[arg(long)]
	pub config: Option<String>,
	/// Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details
	#[arg(short = 'Z', long)]
	pub z: Option<String>,
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
	/// Build artifacts with the specified profile
	#[arg(long)]
	pub profile: Option<String>,
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
	#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
	pub trailing_args: Vec<String>,
}

impl Default for CargoBuildCmd {
	fn default() -> Self { Self::parse_from(&[""]) }
}

impl CargoBuildCmd {
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

	/// Blocking spawn of the cargo build command
	pub fn spawn(&self) -> Result<()> {
		let CargoBuildCmd {
			cmd,
			package,
			bin,
			example,
			test,
			lib,
			doc,
			release,
			target,
			trailing_args,
			message_format,
			verbose,
			quiet,
			color,
			config,
			z,
			features,
			all_features,
			no_default_features,
			jobs,
			keep_going,
			profile,
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

		// Collect args in a vector for printing
		let mut args = Vec::new();
		args.push(cmd.as_str());

		if let Some(pkg) = package {
			args.push("--package");
			args.push(pkg.as_str());
		}
		if let Some(bin) = bin {
			args.push("--bin");
			args.push(bin.as_str());
		}
		if let Some(ex) = example {
			args.push("--example");
			args.push(ex.as_str());
		}
		if let Some(test) = test {
			args.push("--test");
			args.push(test.as_str());
		}
		if *lib {
			args.push("--lib");
		}
		if *doc {
			args.push("--doc");
		}
		if *release {
			args.push("--release");
		}
		if let Some(target) = target {
			args.push("--target");
			args.push(target.as_str());
		}

		if let Some(format) = message_format {
			args.push("--message-format");
			args.push(format.as_str());
		}
		match verbose {
			1 => {
				args.push("-v");
			}
			2 => {
				args.push("-vv");
			}
			n if *n > 2 => {
				args.push("-vvv");
			}
			_ => {}
		}
		if *quiet {
			args.push("--quiet");
		}
		if let Some(color_opt) = color {
			args.push("--color");
			args.push(color_opt.as_str());
		}
		if let Some(config_value) = config {
			args.push("--config");
			args.push(config_value.as_str());
		}
		if let Some(z_flag) = z {
			args.push("-Z");
			args.push(z_flag.as_str());
		}
		if let Some(features_list) = features {
			args.push("--features");
			args.push(features_list.as_str());
		}
		if *all_features {
			args.push("--all-features");
		}
		if *no_default_features {
			args.push("--no-default-features");
		}
		if let Some(jobs_count) = jobs {
			args.push("--jobs");
			args.push(jobs_count.as_str());
		}
		if *keep_going {
			args.push("--keep-going");
		}
		if let Some(profile_name) = profile {
			args.push("--profile");
			args.push(profile_name.as_str());
		}
		if let Some(dir) = target_dir {
			args.push("--target-dir");
			args.push(dir.as_str());
		}
		if *unit_graph {
			args.push("--unit-graph");
		}
		if let Some(timings_format) = timings {
			args.push("--timings");
			args.push(timings_format.as_str());
		}
		if let Some(manifest) = manifest_path {
			args.push("--manifest-path");
			args.push(manifest.as_str());
		}
		if let Some(lockfile) = lockfile_path {
			args.push("--lockfile-path");
			args.push(lockfile.as_str());
		}
		if *ignore_rust_version {
			args.push("--ignore-rust-version");
		}
		if *locked {
			args.push("--locked");
		}
		if *offline {
			args.push("--offline");
		}
		if *frozen {
			args.push("--frozen");
		}

		if !trailing_args.is_empty() {
			args.push("--");
			for arg in trailing_args {
				args.push(arg);
			}
		}

		// Print the command
		println!("Running: cargo {}", args.join(" "));

		// Build and execute command
		let mut command = Command::new("cargo");
		command.args(&args);

		command.status()?.exit_ok()?;
		Ok(())
	}
}

//cargo build -p beet_site --message-format=json | jq -r 'select(.reason == "compiler-artifact" and .target.kind == ["bin"]) | .filenames[]'


#[cfg(test)]
mod test {
	use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	fn works() {
		assert_eq!(CargoBuildCmd::default().cmd, "build");
	}
}
