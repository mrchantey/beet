#![doc = include_str!("../README.md")]

pub mod database;
pub mod embed_model;
pub mod mcp_server;
pub mod mcp_client;
pub mod split_text;



pub mod prelude {
	pub use super::init_tracing;
	pub use crate::database::*;
	pub use crate::embed_model::*;
	pub use crate::mcp_server::*;
	pub use crate::mcp_client::*;
	pub use crate::split_text::*;
}


/// inits tracing
pub fn init_tracing() {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::from_default_env()
				.add_directive(tracing::Level::DEBUG.into()),
		) // ensure logs dont go to stdout which is used by the mcp server
		.with_writer(std::io::stderr)
		// dont format the output
		.with_ansi(false)
		.init();
}
