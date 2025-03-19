use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use sweet::prelude::GracefulChild;

pub struct BuildBinaries<'a> {
	build_cmd: BuildCmd,
	watch_args: &'a WatchArgs,
	exe_path_native: PathBuf,
	/// command for building in wasm mode
	build_wasm: Option<BuildWasm<'a>>,
	server_process: GracefulChild,
	collect_routes: Option<CollectRoutes>,
}


impl<'a> BuildBinaries<'a> {
	pub fn new(
		mut build_cmd: BuildCmd,
		watch_args: &'a WatchArgs,
	) -> Result<Self> {
		if !watch_args.as_static {
			build_cmd.cargo_args = Some("--features beet/server".to_string());
		}

		let exe_path_native = build_cmd.exe_path();

		// here we're compiling once
		let cx = Self::get_cx(&exe_path_native)?;

		let collect_routes = cx
			.file
			.parent()
			.map(|p| {
				p.join("routes")
					.canonicalize()
					.ok()
					.map(|p| if !p.exists() { None } else { Some(p) })
					.flatten()
			})
			.flatten()
			.map(|routes_dir| CollectRoutes {
				routes_dir,
				..Default::default()
			});

		let should_build_wasm = true;

		let build_wasm = if should_build_wasm {
			Some(BuildWasm::new(&build_cmd, watch_args)?)
		} else {
			None
		};


		let this = Self {
			build_cmd,
			exe_path_native,
			watch_args,
			collect_routes,
			build_wasm,
			server_process: GracefulChild::default().as_only_ctrlc_handler(),
		};

		this.recompile_and_reload()?;

		Ok(this)
	}

	/// Simply rerun the process with --static to rebuild templates
	pub fn reload(&self) -> Result<()> {
		if self.watch_args.no_build {
			return Ok(());
		}
		self.build_templates()?;
		Ok(())
	}

	pub fn recompile_and_reload(&self) -> Result<()> {
		if self.watch_args.no_build {
			return Ok(());
		}
		if let Some(collect_routes) = &self.collect_routes {
			// TODO only recollect routes if routes change?
			collect_routes.build_and_write()?;
		}
		self.recompile()?;

		if !self.watch_args.as_static {
			self.server_process.kill();
			let child = self.run_server(&self.watch_args)?;
			self.server_process.set(child);
		}

		if let Some(build_wasm) = &self.build_wasm {
			build_wasm.build()?;
		}
		Ok(())
	}

	pub fn recompile(&self) -> Result<()> {
		println!("🥁 building native");
		self.build_cmd.spawn()?;
		Ok(())
	}

	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files
	pub fn build_templates(&self) -> Result<()> {
		Command::new(&self.exe_path_native)
			.arg("--html-dir")
			.arg(&self.watch_args.html_dir)
			.arg("--static")
			.status()?
			.exit_ok()?;
		Ok(())
	}
	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files
	fn get_cx(exe_path_native: &Path) -> Result<RootContext> {
		let stdout = Command::new(&exe_path_native)
			.arg("--root-context")
			.output()?
			.stdout;
		let cx = ron::de::from_bytes(&stdout)?;
		Ok(cx)
	}

	fn run_server(&self, watch_args: &WatchArgs) -> Result<Child> {
		let child = Command::new(&self.exe_path_native)
			.arg("--html-dir")
			.arg(&watch_args.html_dir)
			// kill child when parent is killed
			.process_group(0)
			.spawn()?;
		Ok(child)
	}
}


struct BuildWasm<'a> {
	build_cmd: BuildCmd,
	exe_path: PathBuf,
	watch_args: &'a WatchArgs,
}

impl<'a> BuildWasm<'a> {
	pub fn new(
		build_native: &BuildCmd,
		watch_args: &'a WatchArgs,
	) -> Result<Self> {
		let mut build_cmd = build_native.clone();
		build_cmd.target = Some("wasm32-unknown-unknown".to_string());
		let exe_path = build_cmd.exe_path();
		let this = Self {
			build_cmd,
			exe_path,
			watch_args,
		};
		Ok(this)
	}

	pub fn build(&self) -> Result<()> {
		println!("🥁 building wasm");
		self.build_cmd.spawn()?;
		self.wasm_bindgen()?;
		Ok(())
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
