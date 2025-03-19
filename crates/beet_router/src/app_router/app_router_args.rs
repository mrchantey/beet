use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;


/// Cli args parser when running an [`AppRouter`].
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct AppRouterArgs {
	/// Only build routes, do not run a server
	#[arg(long = "static")]
	pub is_static: bool,
	/// root for the emitted html files
	#[arg(long, default_value = "target/client")]
	pub html_dir: PathBuf,
}


impl AppRouterArgs {
	/// Check the arguments parsed in match the compiled feature set
	pub fn validate(self) -> Result<Self> {
		#[cfg(not(feature = "server"))]
		if !self.is_static {
			anyhow::bail!("Server feature is required to run a server")
		}
		Ok(self)
	}

	#[cfg(target_arch = "wasm32")]
	pub fn from_url_params() -> Result<Self> {
		// TODO actually parse from search params
		Ok(Self {
			is_static: false,
			html_dir: "".into(),
		})
	}
}
