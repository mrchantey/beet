use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use sweet::prelude::GracefulChild;

/// Performs all steps for a full recompile and reload
pub struct BuildApp;

impl BuildApp {
	pub fn new(
		build_cmd: &BuildCmd,
		watch_args: &WatchArgs,
	) -> Result<BuildStepGroup> {
		let exe_path = build_cmd.exe_path();

		// here we're compiling once
		if !watch_args.only.is_empty() {
			let mut group = BuildStepGroup::default();
			for arg in watch_args.only.iter() {
				match arg.as_str() {
					"setup" => group.add(RunSetup::new(&build_cmd)?),
					"native" => {
						group.add(BuildNative::new(&build_cmd, &watch_args))
					}
					"server" => {
						group.add(RunServer::new(&watch_args, &exe_path))
					}
					"static" => {
						group.add(ExportStatic::new(watch_args, &exe_path))
					}
					"collect-wasm" => group.add(CollectWasmRoutes::default()),
					"build-wasm" => {
						group.add(BuildWasm::new(&build_cmd, &watch_args)?)
					}
					_ => todo!(),
				};
			}
			Ok(group)
		} else {
			let mut group = BuildStepGroup::default();
			group
				// 1. perform setup steps specified in the app
				.add(RunSetup::new(&build_cmd)?)
				// 2. rebuild the native binary
				.add(BuildNative::new(&build_cmd, &watch_args))
				// 3. run the server as soon as its ready
				.add(RunServer::new(&watch_args, &exe_path))
				// 4. export all static files from the app
				//   - html files
				//   - client island entries
				.add(ExportStatic::new(watch_args, &exe_path))
				// 5. create the wasm routes `collect()` function
				.add(CollectWasmRoutes::default())
				// 5. build the wasm binary
				.add(BuildWasm::new(&build_cmd, &watch_args)?);

			Ok(group)
		}
	}
}

/// This step *could* be done in the actual binary instead of passing
/// the config to the cli, but allowing the cli to handle it means
/// we dont need to recompile this step on live reload changes
pub struct RunSetup;

impl RunSetup {
	pub fn new(build_native: &BuildCmd) -> Result<BuildStepGroup> {
		println!("🥁 Build Step 1: Setup");

		let mut build_cmd = build_native.clone();
		build_cmd.cargo_args = Some("--features setup".to_string());
		build_cmd.spawn()?;

		let exe_path = build_cmd.exe_path();

		let stdout = Command::new(&exe_path).output()?.stdout;
		let setup_config: AppConfig = ron::de::from_bytes(&stdout)?;

		let mut group = BuildStepGroup::default();
		for item in setup_config.build_steps.into_iter() {
			group.add(item);
		}
		Ok(group)
	}
}

pub struct BuildNative {
	build_cmd: BuildCmd,
}

impl BuildNative {
	pub fn new(build_cmd: &BuildCmd, watch_args: &WatchArgs) -> Self {
		let mut build_cmd = build_cmd.clone();
		if !watch_args.as_static {
			build_cmd.cargo_args = Some("--features beet/server".to_string());
		}
		Self { build_cmd }
	}
}

impl BuildStep for BuildNative {
	fn run(&self) -> Result<()> {
		println!("🥁 Build Step 2: Native");
		self.build_cmd.run()?;
		Ok(())
	}
}

/// Run the native app with the `--static` flag, exporting client islands and html files
pub struct ExportStatic {
	exe_path: PathBuf,
	watch_args: WatchArgs,
}

impl ExportStatic {
	pub fn new(watch_args: &WatchArgs, exe_path: &Path) -> Self {
		Self {
			watch_args: watch_args.clone(),
			exe_path: exe_path.to_path_buf(),
		}
	}
}

impl BuildStep for ExportStatic {
	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files,
	/// saving them to the `html_dir`
	fn run(&self) -> Result<()> {
		Command::new(&self.exe_path)
			.arg("--html-dir")
			.arg(&self.watch_args.html_dir)
			.arg("--static")
			.status()?
			.exit_ok()?;
		Ok(())
	}
}





pub struct BuildWasm {
	build_cmd: BuildCmd,
	exe_path: PathBuf,
	watch_args: WatchArgs,
}

impl BuildWasm {
	pub fn new(
		build_native: &BuildCmd,
		watch_args: &WatchArgs,
	) -> Result<Self> {
		let mut build_cmd = build_native.clone();
		build_cmd.target = Some("wasm32-unknown-unknown".to_string());
		let exe_path = build_cmd.exe_path();
		let this = Self {
			build_cmd,
			exe_path,
			watch_args: watch_args.clone(),
		};
		Ok(this)
	}

	/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
	/// should be the path to the output of `cargo build`
	fn wasm_bindgen(&self) -> Result<()> {
		Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(self.watch_args.html_dir.join("wasm"))
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
		println!("🥁 Build Step 3: WASM");
		self.build_cmd.spawn()?;
		self.wasm_bindgen()?;
		Ok(())
	}
}


pub struct RunServer {
	exe_path: PathBuf,
	watch_args: WatchArgs,
	child_process: GracefulChild,
}

impl RunServer {
	pub fn new(watch_args: &WatchArgs, exe_path: &Path) -> Self {
		Self {
			watch_args: watch_args.clone(),
			exe_path: exe_path.to_path_buf(),
			child_process: GracefulChild::default().as_only_ctrlc_handler(),
		}
	}
}

impl BuildStep for RunServer {
	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files
	fn run(&self) -> Result<()> {
		if self.watch_args.as_static {
			return Ok(());
		}

		self.child_process.kill();

		let child = Command::new(&self.exe_path)
			.arg("--html-dir")
			.arg(&self.watch_args.html_dir)
			// kill child when parent is killed
			.process_group(0)
			.spawn()?;
		self.child_process.set(child);
		Ok(())
	}
}
