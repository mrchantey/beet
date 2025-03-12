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
	#[command(flatten)]
	watch_args: WatchArgs,
	#[command(flatten)]
	build_template_map: BuildTemplateMap,
	#[command(flatten)]
	build_binaries: BuildBinaries,
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

	async fn watch(self) -> Result<()> {
		TemplateWatcher::new(
			self.build_template_map,
			|| self.build_binaries.run_native(&self.watch_args),
			|| self.build_binaries.recompile(&self.watch_args),
		)?
		.run_once_and_watch()
		.await
	}
}
