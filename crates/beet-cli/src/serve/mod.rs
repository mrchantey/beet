use anyhow::Result;
use beet_router::prelude::*;
use clap::Parser;
use diff_file::RoutesBuilder;
use std::path::PathBuf;
use sweet::prelude::Server;
mod cargo_cmd;
mod diff_file;
///
#[derive(Debug, Parser)]
pub struct Serve {
	#[command(flatten)]
	parse_file_router: BuildRoutesMod,
	/// 🦀 the commands that will be used to build the route1s 🦀
	#[command(flatten)]
	cargo_run: cargo_cmd::CargoCmd,
	/// directory to serve from
	#[arg(short = 'd', long)]
	serve_dir: PathBuf,
}

impl Serve {
	pub async fn run(mut self) -> Result<()> {
		self.parse_file_router.build_and_write()?;

		let watch_handle = RoutesBuilder::new(
			self.parse_file_router.src,
			std::mem::take(&mut self.cargo_run),
		)
		.watch();

		println!("🥁 Server running at {}", &self.serve_dir.display());
		let server = Server {
			dir: self.serve_dir.clone(),
			..Default::default()
		};

		server.run().await?;
		watch_handle.abort();

		Ok(())
	}
}
