use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Child;
use std::process::Command;
use sweet::prelude::GracefulChild;

/// Build both the server and wasm client binaries
/// for a beet app.

/// Serve a html application as either a spa or mpa
#[derive(Debug, Parser)]
pub struct BuildBinariesArgs {
	/// If the site contains reactivity, also build the client side wasm
	// TODO automate by checking for client-load
	#[arg(long)]
	wasm: bool,
	/// 🦀 the commands that will be used to build the html files 🦀
	#[command(flatten)]
	build_cmd: BuildCmd,
}


impl BuildBinariesArgs {
	pub fn into_runner(self, watch_args: &WatchArgs) -> Result<BuildBinaries> {
		BuildBinaries::new(self, watch_args)
	}
}


pub struct BuildBinaries<'a> {
	build_args: BuildBinariesArgs,
	watch_args: &'a WatchArgs,
	exe_path_native: PathBuf,
	exe_path_wasm: PathBuf,
	/// command for building in wasm mode
	build_wasm_cmd: BuildCmd,
	server_process: GracefulChild,
	collect_routes: Option<CollectRoutes>,
}


impl<'a> BuildBinaries<'a> {
	pub fn new(
		mut build_args: BuildBinariesArgs,
		watch_args: &'a WatchArgs,
	) -> Result<Self> {
		if !watch_args.as_static {
			build_args.build_cmd.cargo_args =
				Some("--features beet/server".to_string());
		}

		let exe_path_native = build_args.build_cmd.exe_path();
		let mut build_wasm = build_args.build_cmd.clone();
		build_wasm.target = Some("wasm32-unknown-unknown".to_string());
		let exe_path_wasm = build_wasm.exe_path();


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



		let this = Self {
			exe_path_native,
			exe_path_wasm,
			build_args,
			watch_args,
			collect_routes,
			build_wasm_cmd: build_wasm,
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

		if self.build_args.wasm {
			println!("🥁 building wasm");
			self.build_wasm()?;
		}
		Ok(())
	}



	fn recompile(&self) -> Result<()> {
		println!("🥁 building native");
		self.build_args.build_cmd.spawn()?;
		Ok(())
	}



	/// run the built binary with the `--static` flag, instructing
	/// it to not spin up a server, and instead just build the static files
	fn build_templates(&self) -> Result<()> {
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


	/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
	/// should be the path to the output of `cargo build`
	fn build_wasm(&self) -> Result<()> {
		self.build_wasm_cmd.spawn()?;

		Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(self.watch_args.html_dir.join("wasm"))
			.arg("--out-name")
			.arg("bindgen")
			.arg("--target")
			.arg("web")
			.arg("--no-typescript")
			.arg(&self.exe_path_wasm)
			.status()?
			.exit_ok()?;

		// TODO wasm-opt in release

		Ok(())
	}
}
