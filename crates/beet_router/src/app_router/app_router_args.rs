use clap::Parser;
use std::path::PathBuf;


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
	/// reexport of [`clap::Parser::parse`]
	pub fn parse() -> Self { Parser::parse() }

	#[cfg(target_arch = "wasm32")]
	pub fn from_url_params() -> anyhow::Result<Self> {
		// TODO actually parse from search params
		Ok(Self {
			is_static: false,
			html_dir: "".into(),
		})
	}
}
