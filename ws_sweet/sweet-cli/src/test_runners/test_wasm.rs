use anyhow::Result;
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use std::str::FromStr;
use sweet::prelude::*;

/// The wasm test runner
///
/// To use add the following:
///
/// ```toml
///
/// # .cargo/config.toml
///
/// [target.wasm32-unknown-unknown]
///
/// runner = 'sweet test-wasm'
///
/// ```
///
#[derive(Debug, Parser)]
pub struct TestWasm {
	/// the file passed in by cargo test.
	///
	/// It will look something like $CARGO_TARGET_DIR/wasm32-unknown-unknown/debug/deps/hello_test-c3298911e67ad05b.wasm
	test_binary: String,
	/// arguments passed to wasm-bindgen
	#[arg(long)]
	wasm_bindgen_args: Option<String>,

	// we wont actuallly use this because the args will
	// be passed to deno, but it provides --help messages
	#[command(flatten)]
	runner_args: sweet::prelude::TestRunnerConfig,
}


impl TestWasm {
	pub fn run(self) -> Result<()> {
		self.run_wasm_bindgen()?;
		self.init_deno()?;
		self.run_deno()?;
		Ok(())
	}
	fn run_wasm_bindgen(&self) -> Result<()> {
		let output = Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(sweet_target_dir())
			.arg("--out-name")
			.arg("bindgen")
			.arg("--target")
			.arg("web")
			.arg("--no-typescript")
			.arg(&self.test_binary)
			.args(
				self.wasm_bindgen_args
					.as_deref()
					.unwrap_or_default()
					.split_whitespace(),
			)
			.spawn()?;

		handle_process("wasm-bindgen", output)?;
		Ok(())
	}


	/// Move the deno file to the correct directory,
	/// if this is the first time this will also ensure deno is installed
	/// by running `deno --version`
	fn init_deno(&self) -> Result<()> {
		let deno_runner_path = deno_runner_path();
		let deno_str = include_str!("./deno.ts");

		// ⚠️ we should check the hash here
		if ReadFile::exists(&deno_runner_path) {
			let runner_hash = ReadFile::hash_file(&deno_runner_path)?;
			let deno_hash = ReadFile::hash_string(deno_str);
			if runner_hash == deno_hash {
				return Ok(());
			}
		};

		let deno_installed =
			match Command::new("deno").arg("--version").status() {
				Ok(val) => val.success(),
				_ => false,
			};
		if !deno_installed {
			anyhow::bail!(INSTALL_DENO);
		}
		println!("copying deno file to {}", deno_runner_path.display());

		// wasm-bindgen will ensure parent dir exists
		fs::write(deno_runner_path, deno_str)?;
		Ok(())
	}

	fn run_deno(&self) -> Result<()> {
		// args will look like this so skip 3
		// sweet test-wasm binary-path *actual-args
		// why doesnt it work with three?
		let args = std::env::args().skip(2).collect::<Vec<_>>();
		let child = Command::new("deno")
			.arg("--allow-read")
			.arg("--allow-net")
			.arg("--allow-env")
			.arg(deno_runner_path())
			.args(args)
			.spawn()?;
		handle_process("deno", child)?;
		Ok(())
	}
}


fn handle_process(_stderr_prefix: &str, child: Child) -> Result<()> {
	let output = child.wait_with_output()?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	if !stdout.is_empty() {
		println!("{}", stdout);
	}

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		anyhow::bail!("{stderr}");
	}
	Ok(())
}

fn workspace_root() -> PathBuf {
	let root_str = std::env::var("SWEET_ROOT").unwrap_or_default();
	PathBuf::from_str(&root_str).unwrap()
}

fn sweet_target_dir() -> PathBuf {
	let mut path = workspace_root();
	path.push("target/sweet");
	path
}

fn deno_runner_path() -> PathBuf {
	let mut path = sweet_target_dir();
	path.push("deno.ts");
	path
}

const INSTALL_DENO: &str = "
🦖 Sweet uses Deno for wasm tests 🦖

Install Deno via:
shell: 				curl -fsSL https://deno.land/install.sh | sh
powershell: 	irm https://deno.land/install.ps1 | iex
website: 			https://docs.deno.com/runtime/getting_started/installation/

";


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	use test_wasm::deno_runner_path;

	#[test]
	fn works() {
		expect(deno_runner_path().to_string_lossy().replace("\\", "/"))
			.to_end_with("target/sweet/deno.ts");
	}
}
