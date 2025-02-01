use anyhow::Result;
use clap::Parser;
mod cargo_run;

///
#[derive(Debug, Parser)]
pub struct Serve {
	/// Path to the source directory
	///
	#[arg(long, default_value = "src")]
	src: String,
	/// 🦀 cargo run args 🦀
	#[command(flatten)]
	cargo_run: cargo_run::CargoRun,
}

impl Serve {
	pub fn run(self) -> Result<()> {
		self.cargo_run.run()?;

		Ok(())
	}
}
