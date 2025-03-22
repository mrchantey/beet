use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use sweet::prelude::Server;


/// Watch the beet project and rebuild on changes
#[derive(Debug, Parser)]
pub struct Watch {
	#[command(flatten)]
	watch_args: WatchArgs,
	#[command(flatten)]
	build_template_map: BuildTemplateMap,
	#[command(flatten)]
	build_cmd: BuildCmd,
}


#[derive(Debug, Clone, Parser)]
pub struct WatchArgs {
	/// Do not build any binaries, just rebuild the templates
	#[arg(long)]
	pub no_build: bool,
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	pub as_static: bool,
	/// root for the emitted html files
	#[arg(long, default_value = "target/client")]
	pub html_dir: PathBuf,
	/// Only execute the provided build steps,
	/// options are `setup`, `native`, `server`, `static`, `collect-wasm` `build-wasm`
	#[arg(long, value_delimiter = ',')]
	pub only: Vec<String>,
}

impl Watch {
	pub async fn run(self) -> Result<()> {
		if self.watch_args.as_static {
			self.watch_and_serve().await
		} else {
			self.watch().await
		}
	}

	/// Run in static mode, building the site and serving it
	async fn watch_and_serve(self) -> Result<()> {
		let watch_args = self.watch_args.clone();
		let watch_handle = tokio::spawn(async move {
			self.watch().await.map_err(|e| {
				// watcher errors are fatal, print the error and exit
				eprintln!("{:#?}", e);
				std::process::exit(1);
			})
		});
		// run a simple file server with live-reload on change
		Server {
			dir: watch_args.html_dir,
			no_clear: true,
			..Default::default()
		}
		.run()
		.await?;
		watch_handle.abort();
		Ok(())
	}

	/// Create a template watcher that will
	/// 1. rebuild templates on any file change
	/// 2. recompile on code changes
	/// 3. run the process
	async fn watch(self) -> Result<()> {
		let build_app = BuildApp::new(&self.build_cmd, &self.watch_args)?;

		let build_templates =
			ExportStatic::new(&self.watch_args, &self.build_cmd.exe_path());

		TemplateWatcher::new(
			self.build_template_map,
			|| build_templates.run(),
			|| build_app.run(),
		)?
		.run_once_and_watch()
		.await
	}
}
