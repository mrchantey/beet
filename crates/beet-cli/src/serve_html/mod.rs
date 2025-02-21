mod cargo_cmd;
use anyhow::Result;
use beet_router::prelude::BuildRsxTemplateMap;
use beet_router::prelude::CollectRoutes;
use beet_router::prelude::TemplateWatcher;
pub use cargo_cmd::*;
use clap::Parser;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use sweet::prelude::Server;


/// Serve a html application as either a spa or mpa
#[derive(Debug, Parser)]
pub struct ServeHtml {
	#[arg(long)]
	mpa: bool,
	/// If the site contains reactivity, also build the client side wasm
	#[arg(long)]
	wasm: bool,
	/// if --mpa is passed, serve as a multi-page application
	/// using file-based routes
	#[command(flatten)]
	collect_routes: CollectRoutes,
	/// 🦀 the commands that will be used to build the html files 🦀
	#[command(flatten)]
	cargo_cmd: CargoCmd,
	/// directory to serve from
	#[arg(short = 'd', long)]
	serve_dir: PathBuf,
}

impl ServeHtml {
	pub async fn run(self) -> Result<()> {
		let serve_dir = self.serve_dir.clone();
		let watch_handle = tokio::spawn(async move {
			self.watch().await.map_err(|e| {
				// if the watcher failed, print the error and exit
				eprintln!("{:#?}", e);
				std::process::exit(1);
			})
		});

		println!("🥁 Server running at {}", serve_dir.display());
		let server = Server {
			dir: serve_dir,
			no_clear: true,
			..Default::default()
		};


		server.run().await?;
		watch_handle.abort();

		Ok(())
	}

	/// Runs a [`TemplateWatcher`] with the following functions:
	/// - `reload` - just run the executable again, usually rebuilding html files
	/// - `recompile_and_reload` - recompile the executable, then reload
	async fn watch(self) -> Result<()> {
		// todo we're misusing the collect_routes src, we should use a top level
		let src_dir = &self.collect_routes.src;
		let build_templates = BuildRsxTemplateMap::new(&src_dir);

		let build_wasm_cmd = if self.wasm {
			let mut cmd = self.cargo_cmd.clone();
			cmd.target = Some("wasm32-unknown-unknown".to_string());
			Some(cmd)
		} else {
			None
		};

		let exe_path = self.cargo_cmd.exe_path();
		// println!("on reload running:\n{}", exe_path.display());
		let reload = || -> Result<()> {
			Command::new(&exe_path).status()?.exit_ok()?;
			Ok(())
		};

		let recompile_and_reload = move || -> Result<()> {
			if self.mpa {
				// TODO only recollect routes if routes change?
				self.collect_routes.build_and_write()?;
			}
			println!("🥁 building native");
			self.cargo_cmd.spawn()?;
			if let Some(wasm_cmd) = &build_wasm_cmd {
				println!("🥁 building wasm");
				wasm_cmd.spawn()?;
				self.wasm_bindgen(&wasm_cmd.exe_path())?;
			}
			reload()?;
			Ok(())
		};

		// run once before watching
		build_templates.build_and_write()?;
		recompile_and_reload()?;

		// always compile on first run
		TemplateWatcher::new(build_templates, reload, recompile_and_reload)?
			.watch()
			.await
	}

	/// execute `wasm-bindgen` with inferred locations. The wasm_exe_path
	/// should be the path to the output of `cargo build`
	fn wasm_bindgen(&self, wasm_exe_path: &Path) -> Result<()> {
		Command::new("wasm-bindgen")
			.arg("--out-dir")
			.arg(self.serve_dir.join("wasm"))
			.arg("--out-name")
			.arg("bindgen")
			.arg("--target")
			.arg("web")
			.arg("--no-typescript")
			.arg(wasm_exe_path)
			.status()?
			.exit_ok()?;
		Ok(())
	}
}
