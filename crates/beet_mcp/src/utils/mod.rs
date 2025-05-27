mod rig_mcp_adapter;
pub use rig_mcp_adapter::*;
mod model;
pub use model::*;



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
