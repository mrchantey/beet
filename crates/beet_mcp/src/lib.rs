#![doc = include_str!("../README.md")]

pub mod crate_rag;
pub mod database;
pub mod index_repository;
pub mod mcp_client;
pub mod mcp_server;
pub mod model;
pub mod rig_mcp_adapter;
pub mod split_text;



pub mod prelude {
	pub use super::init_tracing;
	pub use crate::crate_rag::*;
	pub use crate::database::*;
	pub use crate::index_repository::*;
	pub use crate::mcp_client::*;
	pub use crate::mcp_server::*;
	pub use crate::model::*;
	pub use crate::rig_mcp_adapter::*;
	pub use crate::split_text::*;
}


/// inits tracing
pub fn init_tracing(level: tracing::Level) {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::from_default_env()
				.add_directive(level.into()),
		) // ensure logs dont go to stdout which is used by the mcp server
		.with_writer(std::io::stderr)
		// dont format the output
		.with_ansi(false)
		.init();
}
