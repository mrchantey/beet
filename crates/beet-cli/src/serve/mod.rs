use anyhow::Result;
use beet_router::prelude::*;
use clap::Parser;
use file_routes_watcher::FileRoutesWatcher;
use std::path::PathBuf;
use sweet::prelude::Server;
mod cargo_cmd;
mod file_routes_watcher;
///
#[derive(Debug, Parser)]
pub struct Serve {
	#[command(flatten)]
	collect_routes: CollectRoutes,
	/// 🦀 the commands that will be used to build the route1s 🦀
	#[command(flatten)]
	cargo_run: cargo_cmd::CargoCmd,
	/// directory to serve from
	#[arg(short = 'd', long)]
	serve_dir: PathBuf,
}

impl Serve {
	pub async fn run(self) -> Result<()> {
		let Serve {
			collect_routes,
			cargo_run,
			serve_dir,
		} = self;

		let watch_handle = tokio::spawn(async move {
			FileRoutesWatcher::new(collect_routes, cargo_run)?
				.watch()
				.await
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
}
