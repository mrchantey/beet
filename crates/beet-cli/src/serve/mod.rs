use anyhow::Result;
use beet_router::prelude::*;
use clap::Parser;
use sweet::prelude::FsWatcher;
use sweet::prelude::Server;
mod cargo_run;
///
#[derive(Debug, Parser)]
pub struct Serve {
	#[command(flatten)]
	parse_file_router: ParseRoutesDir,
	/// 🦀 cargo run args 🦀
	#[command(flatten)]
	cargo_run: cargo_run::CargoRun,
	/// directory to serve from
	#[arg(short = 'd', long)]
	serve_dir: String,
}

impl Serve {
	pub async fn run(mut self) -> Result<()> {
		self.parse_file_router.build_and_write()?;

		let cargo_run = std::mem::take(&mut self.cargo_run);
		let watch_dir = self.parse_file_router.src;
		tokio::spawn(async move {
			FsWatcher::new()
				.with_path(watch_dir.to_string_lossy().to_string())
				.watch_async(|_| {
					cargo_run.run()?;
					Ok(())
				})
				.await
				.unwrap();
		});

		println!("🚀 Server running at {}", &self.serve_dir);
		let server = Server {
			dir: self.serve_dir.clone(),
			..Default::default()
		};

		server.run().await?;
		Ok(())
	}
}
