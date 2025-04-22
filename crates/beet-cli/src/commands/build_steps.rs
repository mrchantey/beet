use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use sweet::prelude::GracefulChild;

pub struct BuildNative {
	build_cmd: CargoBuildCmd,
}

impl BuildNative {
	pub fn new(build_cmd: &CargoBuildCmd, watch_args: &BuildArgs) -> Self {
		let mut build_cmd = build_cmd.clone();
		if !watch_args.as_static {
			build_cmd.cargo_args = Some("--features beet/server".to_string());
		}
		Self { build_cmd }
	}
}

impl BuildStep for BuildNative {
	fn run(&self) -> Result<()> {
		println!("🌱 Build Step 1: Native");
		self.build_cmd.run()?;
		Ok(())
	}
}

/// Run the native app with the `--static` flag, exporting client islands and html files
pub struct ExportStatic {
	exe_path: PathBuf,
	build_args: BuildArgs,
}

impl ExportStatic {
	pub fn new(build_args: &BuildArgs, exe_path: &Path) -> Self {
		Self {
			build_args: build_args.clone(),
			exe_path: exe_path.to_path_buf(),
		}
	}
}

impl BuildStep for ExportStatic {
	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files,
	/// saving them to the `html_dir`
	fn run(&self) -> Result<()> {
		println!(
			"🌱 Build Step 2: HTML \nExecuting {}",
			self.exe_path.display()
		);
		Command::new(&self.exe_path)
			.arg("--html-dir")
			.arg(&self.build_args.html_dir)
			.arg("--static")
			.status()?
			.exit_ok()?;
		Ok(())
	}
}


pub struct BuildWasm {
	build_cmd: CargoBuildCmd,
	exe_path: PathBuf,
	build_args: BuildArgs,
}

impl BuildWasm {
	pub fn new(
		build_native: &CargoBuildCmd,
		build_args: &BuildArgs,
	) -> Result<Self> {
		let mut build_cmd = build_native.clone();
		build_cmd.target = Some("wasm32-unknown-unknown".to_string());
		let exe_path = build_cmd.exe_path();
		let this = Self {
			build_cmd,
			exe_path,
			build_args: build_args.clone(),
		};
		Ok(this)
	}

	/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
	/// should be the path to the output of `cargo build`
	fn wasm_bindgen(&self) -> Result<()> {
		Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(self.build_args.html_dir.join("wasm"))
			.arg("--out-name")
			.arg("bindgen")
			.arg("--target")
			.arg("web")
			.arg("--no-typescript")
			.arg(&self.exe_path)
			.status()?
			.exit_ok()?;

		// TODO wasm-opt in release

		Ok(())
	}
}

impl BuildStep for BuildWasm {
	fn run(&self) -> Result<()> {
		println!("🌱 Build Step 3: WASM");
		self.build_cmd.spawn()?;
		self.wasm_bindgen()?;
		Ok(())
	}
}


pub struct RunServer {
	exe_path: PathBuf,
	build_args: BuildArgs,
	child_process: GracefulChild,
}

impl RunServer {
	pub fn new(build_args: &BuildArgs, exe_path: &Path) -> Self {
		Self {
			build_args: build_args.clone(),
			exe_path: exe_path.to_path_buf(),
			child_process: GracefulChild::default().as_only_ctrlc_handler(),
		}
	}
}

impl BuildStep for RunServer {
	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files
	fn run(&self) -> Result<()> {
		if self.build_args.as_static {
			return Ok(());
		}

		self.child_process.kill();

		let child = Command::new(&self.exe_path)
			.arg("--html-dir")
			.arg(&self.build_args.html_dir)
			// kill child when parent is killed
			.process_group(0)
			.spawn()?;
		self.child_process.set(child);
		Ok(())
	}
}
