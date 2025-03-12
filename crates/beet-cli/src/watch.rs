use std::path::PathBuf;

use crate::prelude::*;
use anyhow::Result;
use beet_router::prelude::BuildTemplateMap;
use beet_router::prelude::TemplateWatcher;
use clap::Parser;
use sweet::prelude::Server;


/// A general watch command with several capabilities:
/// - `Build Templates`: Watch a file or directory and build a `rsx-templates.ron` file
///
#[derive(Debug, Parser)]
pub struct Watch {
	/// Run a simple file server in this process instead of
	/// spinning up the native binary with the --server feature
	#[arg(long = "static")]
	pub as_static: bool,
	/// root for the emitted html files
	#[arg(long, default_value = "target/client")]
	html_dir: PathBuf,
	#[command(flatten)]
	build_template_map: BuildTemplateMap,
	#[command(flatten)]
	build_binaries: BuildBinaries,
}

impl Watch {
	pub async fn run(self) -> Result<()> {
		if self.as_static {
			self.run_static().await
		} else {
			self.run_server().await
		}
	}

	/// Run in static mode, building the site and serving it
	async fn run_static(self) -> Result<()> {
		let html_dir = self.html_dir.clone();
		let watch_handle = tokio::spawn(async move {
			self.watch().await.map_err(|e| {
				// watcher errors are fatal, print the error and exit
				eprintln!("{:#?}", e);
				std::process::exit(1);
			})
		});
		// run a simple file server with live-reload on change
		println!("🥁 Server running at {}", html_dir.display());
		Server {
			dir: html_dir,
			no_clear: true,
			..Default::default()
		}
		.run()
		.await?;
		watch_handle.abort();
		Ok(())
	}

	async fn watch(self) -> Result<()> {
		TemplateWatcher::new(
			self.build_template_map,
			|| self.build_binaries.run_native(),
			|| self.build_binaries.recompile(&self.html_dir),
		)?
		.run_once_and_watch()
		.await
	}
	async fn run_server(self) -> Result<()> {
		// self
		Ok(())
	}
}
