use anyhow::Result;
use beet_router::prelude::*;
use clap::Parser;
mod cargo_run;
///
#[derive(Debug, Parser)]
pub struct Serve {
	#[command(flatten)]
	parse_page_router: ParseFileRouter,
	/// 🦀 cargo run args 🦀
	#[command(flatten)]
	cargo_run: cargo_run::CargoRun,
}

impl Serve {
	pub fn run(self) -> Result<()> {
		let str = self.parse_page_router.build_string()?;
		println!("{}", str);

		// self.cargo_run.run()?;

		Ok(())
	}
}
