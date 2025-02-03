use clap::Parser;
use std::process::Command;


/// Verbatim clone of cargo run args
#[derive(Debug, Clone, Parser)]
#[command(name = "cargo-run")]
pub struct CargoRun {
	/// Error format
	#[arg(long)]
	message_format: Option<String>,
	/// Use verbose output (-vv very verbose/build.rs output)
	#[arg(short = 'v', long, action = clap::ArgAction::Count)]
	verbose: u8,
	/// Do not print cargo log messages
	#[arg(short, long)]
	quiet: bool,
	/// Coloring: auto, always, never
	#[arg(long)]
	color: Option<String>,
	/// Override a configuration value
	config: Option<String>,
	/// Unstable (nightly-only) flags to Cargo, see 'cargo -Z help' for details
	#[arg(short = 'Z', long)]
	z: Option<String>,
	/// Package with the target to run
	#[arg(short = 'p', long = "package")]
	package: Option<String>,
	/// Name of the bin target to run
	#[arg(long)]
	bin: Option<String>,
	/// Name of the example target to run
	#[arg(long)]
	example: Option<String>,
	/// Space or comma separated list of features to activate
	#[arg(short = 'F', long = "features")]
	features: Option<String>,
	/// Activate all available features
	#[arg(long)]
	all_features: bool,
	/// Do not activate the `default` feature
	#[arg(long)]
	no_default_features: bool,
	/// Number of parallel jobs, defaults to # of CPUs.
	#[arg(short = 'j', long)]
	jobs: Option<String>,
	/// Do not abort the build as soon as there is an error
	#[arg(long)]
	keep_going: bool,
	/// Build artifacts in release mode, with optimizations
	#[arg(long)]
	release: bool,
	/// Build artifacts with the specified profile
	#[arg(long)]
	profile: Option<String>,
	/// Build for the target triple
	#[arg(long)]
	target: Option<String>,
	/// Directory for all generated artifacts
	#[arg(long)]
	target_dir: Option<String>,
	/// Output build graph in JSON (unstable)
	#[arg(long)]
	unit_graph: bool,
	/// Timing output formats (unstable) (comma separated): html, json
	#[arg(long)]
	timings: Option<String>,
	/// Path to Cargo.toml
	#[arg(long)]
	manifest_path: Option<String>,
	/// Path to Cargo.lock (unstable)
	#[arg(long)]
	lockfile_path: Option<String>,
	/// Ignore `rust-version` specification in packages
	#[arg(long)]
	ignore_rust_version: bool,
	/// Assert that `Cargo.lock` will remain unchanged
	#[arg(long)]
	locked: bool,
	/// Run without accessing the network
	#[arg(long)]
	offline: bool,
	/// Equivalent to specifying both --locked and --offline
	#[arg(long)]
	frozen: bool,
	/// Arguments for the binary or example to run
	#[arg(trailing_var_arg = true)]
	args: Vec<String>,
}

impl Default for CargoRun {
	fn default() -> Self { Self::parse_from(&[""]) }
}

impl CargoRun {
	pub fn run(&self) -> std::io::Result<()> {
		let mut cmd = Command::new("cargo");
		cmd.arg("run");

		let CargoRun {
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

		cmd.status()?;
		Ok(())
	}
}
