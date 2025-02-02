use anyhow::Result;
use beet_router::prelude::*;
use clap::Parser;
mod cargo_run;
///
#[derive(Debug, Parser)]
pub struct Serve {
	#[command(flatten)]
	parse_file_router: ParseFileRouter,
	/// 🦀 cargo run args 🦀
	#[command(flatten)]
	cargo_run: cargo_run::CargoRun,
}

impl Serve {
	pub fn run(self) -> Result<()> {
		self.parse_file_router.build_and_write()?;

		// self.cargo_run.run()?;

		Ok(())
	}
}
