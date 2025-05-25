#![doc = include_str!("../README.md")]

pub mod database;
pub mod mcp_server;
pub mod split_text;


pub mod prelude {
	pub use super::init_env;
	pub use crate::database::*;
	pub use crate::mcp_server::*;
	pub use crate::split_text::*;
}


/// init tracing and dotenv
pub fn init_env() {
	dotenv::dotenv().ok();
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::from_default_env()
				.add_directive(tracing::Level::DEBUG.into()),
		) // ensure logs dont go to stdout which is used by the mcp server
		.with_writer(std::io::stderr)
		// dont format the output
		.with_ansi(false)
		.init();
	tracing::info!("initializing environment");
}
