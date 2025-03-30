use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use sweet::prelude::Server;


/// Watch the beet project and rebuild on changes
#[derive(Debug, Parser)]
pub struct RunWatch {
	#[command(flatten)]
	watch_args: WatchArgs,
	#[command(flatten)]
	build_template_map: BuildTemplateMap,
	#[command(flatten)]
	build_cmd: CargoBuildCmd,
}


#[derive(Debug, Clone, Parser)]
pub struct WatchArgs {
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

impl RunWatch {
	pub async fn run(self) -> Result<()> {
		if self.watch_args.as_static {
			self.watch_and_serve().await
		} else {
			self.watch().await
		}
	}

	/// Run in static mode, building the site and serving it via cli
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
		let Self {
			watch_args,
			build_template_map,
			build_cmd,
		} = self;


		let templates_root_dir = build_template_map.templates_root_dir.clone();

		let recompile = RunBuild {
			build_cmd: build_cmd.clone(),
			watch_args: watch_args.clone(),
			build_template_map: build_template_map.clone(),
			server: true,
		}
		.into_group()?;

		let reload = BuildStepGroup::default()
			.with(build_template_map.clone())
			.with(ExportStatic::new(&watch_args, &build_cmd.exe_path()));

		TemplateWatcher::new(
			templates_root_dir,
			|| reload.run(),
			|| recompile.run(),
		)?
		.run_once_and_watch()
		.await
	}
}
