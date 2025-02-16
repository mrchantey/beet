use anyhow::Result;
use beet_router::prelude::*;
use clap::Parser;
use routes_builder::RoutesBuilder;
use std::path::PathBuf;
use sweet::prelude::Server;
mod cargo_cmd;
mod routes_builder;
///
#[derive(Debug, Parser)]
pub struct Serve {
	#[command(flatten)]
	build_routes_mod: BuildRoutesMod,
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
			build_routes_mod,
			cargo_run,
			serve_dir,
		} = self;

		let watch_handle = tokio::spawn(async move {
			RoutesBuilder::new(build_routes_mod, cargo_run)?
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
