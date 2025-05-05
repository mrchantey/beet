use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use sweet::prelude::GracefulChild;


pub struct BuildCodegenNative {
	build_args: BuildArgs,
}

impl BuildCodegenNative {
	pub fn new(build_args: &BuildArgs) -> Self {
		Self {
			build_args: build_args.clone(),
		}
	}
}

impl BuildStep for BuildCodegenNative {
	fn run(&self) -> Result<()> {
		println!("🌱 Running native codegen");
		// FsExt::remove(&html_dir).ok();
		BeetConfig::from_file(&self.build_args.config)?
			.xpipe(BeetConfigToNativeCodegen)?;
		Ok(())
	}
}


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
		println!("🌱 Compiling native binary");
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
			"🌱 Running native binary to generate static files \nExecuting {}",
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


pub struct BuildCodegenWasm {
	build_args: BuildArgs,
}

impl BuildCodegenWasm {
	pub fn new(build_args: &BuildArgs) -> Self {
		Self {
			build_args: build_args.clone(),
		}
	}
}

impl BuildStep for BuildCodegenWasm {
	fn run(&self) -> Result<()> {
		println!("🌱 Running wasm codegen");
		BeetConfig::from_file(&self.build_args.config)?
			.xpipe(BeetConfigToWasmCodegen)?;
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
			// alternatively es modules target: experimental-nodejs-module
			.arg("--no-typescript")
			.arg(&self.exe_path)
			.status()?
			.exit_ok()?
			.xok()
	}

	// TODO wasm opt
	fn wasm_opt(&self) -> Result<()> {
		println!("🌱 Optimizing wasm binary");
		let out_file = self
			.build_args
			.html_dir
			.join("wasm")
			.join("bindgen_bg.wasm");

		let size_before = std::fs::metadata(&out_file)?.len();

		Command::new("wasm-opt")
			.arg("-Oz")
			.arg("--output")
			.arg(&out_file)
			.arg(&out_file)
			.status()?
			.exit_ok()?;

		let size_after = std::fs::metadata(&out_file)?.len();
		println!(
			"🌱 Reduced wasm binary from {} to {}",
			format!("{} KB", size_before as usize / 1024),
			format!("{} KB", size_after as usize / 1024)
		);

		Ok(())
	}
}

impl BuildStep for BuildWasm {
	fn run(&self) -> Result<()> {
		println!("🌱 Compiling wasm binary");
		self.build_cmd.spawn()?;
		self.wasm_bindgen()?;
		if self.build_cmd.release {
			self.wasm_opt()?;
		}
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
