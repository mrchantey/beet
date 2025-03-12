use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;


/// Cli args parser when running a beet app
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct BeetAppArgs {
	/// Only build routes, do not run a server
	#[arg(long = "static")]
	pub is_static: bool,
	/// root for the emitted html files
	#[arg(long, default_value = "target/client")]
	pub html_dir: PathBuf,
}


impl BeetAppArgs {
	/// Check the arguments parsed in match the compiled feature set
	pub fn validate(self) -> Result<Self> {
		#[cfg(not(feature = "server"))]
		if !self.is_static {
			anyhow::bail!("Server feature is required to run a server")
		}
		Ok(self)
	}
}
