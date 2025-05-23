use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

/// Verbatim clone of cargo build/run args
#[derive(Debug, Clone, Parser)]
pub struct CargoBuildCmd {
	/// The top level command to run: `build`, `run`, `test`, etc.
	#[arg(default_value = "build")]
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
	/// used by watcher to also build for wasm
	#[arg(long)]
	pub target: Option<String>,
	/// Any additional arguments passed to cargo
	#[arg(long)]
	cargo_args: Option<String>,
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

	pub fn push_cargo_args(&mut self, args: impl Into<String>) {
		if let Some(cargo_args) = &mut self.cargo_args {
			cargo_args.push(' ');
			cargo_args.push_str(&args.into());
		} else {
			self.cargo_args = Some(args.into());
		}
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
			cargo_args,
		} = self;
		let mut command = Command::new("cargo");
		command.arg(cmd);

		if let Some(pkg) = package {
			command.arg("--package").arg(pkg);
		}
		if let Some(bin) = bin {
			command.arg("--bin").arg(bin);
		}
		if let Some(ex) = example {
			command.arg("--example").arg(ex);
		}
		if let Some(test) = test {
			command.arg("--test").arg(test);
		}
		if *lib {
			command.arg("--lib");
		}
		if *doc {
			command.arg("--doc");
		}
		if *release {
			command.arg("--release");
		}
		if let Some(target) = target {
			command.arg("--target").arg(target);
		}
		if let Some(args) = cargo_args {
			command.arg("--");
			command.args(args.split_whitespace());
		}


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
