use anyhow::Result;
use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

/// Verbatim clone of cargo run args
#[derive(Debug, Clone, Parser)]
pub struct CargoBuildCmd {
	/// Package with the target to run
	#[arg(short = 'p', long = "package")]
	pub package: Option<String>,
	/// Name of the bin target to run
	#[arg(long)]
	pub bin: Option<String>,
	/// Name of the example target to run
	#[arg(long)]
	pub example: Option<String>,
	/// Build artifacts in release mode, with optimizations
	#[arg(long)]
	pub release: bool,
	/// Any additional arguments passed to cargo
	#[arg(long)]
	pub cargo_args: Option<String>,
	/// used by watcher to also build for wasm
	pub target: Option<String>,
}

impl Default for CargoBuildCmd {
	fn default() -> Self { Self::parse_from(&[""]) }
}


impl BuildStep for CargoBuildCmd {
	fn run(&self) -> Result<()> {
		self.spawn()?;
		Ok(())
	}
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
			package,
			bin,
			example,
			release,
			target,
			cargo_args,
		} = self;
		let mut cmd = Command::new("cargo");
		cmd.arg("build");

		if let Some(pkg) = package {
			cmd.arg("--package").arg(pkg);
		}

		if let Some(bin) = bin {
			cmd.arg("--bin").arg(bin);
		}

		if let Some(ex) = example {
			cmd.arg("--example").arg(ex);
		}

		if *release {
			cmd.arg("--release");
		}

		if let Some(target) = target {
			cmd.arg("--target").arg(target);
		}

		if let Some(args) = cargo_args {
			cmd.args(args.split_whitespace());
		}


		cmd.status()?.exit_ok()?;
		Ok(())
	}
}

//cargo build -p beet_site --message-format=json | jq -r 'select(.reason == "compiler-artifact" and .target.kind == ["bin"]) | .filenames[]'
