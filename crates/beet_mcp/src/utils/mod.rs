mod rig_mcp_adapter;
pub use rig_mcp_adapter::*;
mod model;
pub use model::*;


// let current_level = tracing::level_filters::LevelFilter::current();
// println!("Current tracing level: {}", current_level);

/// inits tracing with BEET_LOG, defaulting to the given level.
/// BEET_LOG is used by agents to propagate the log level to the mcp server.
pub fn init_tracing(level: tracing::Level) {
	let level = std::env::var("BEET_LOG")
		.ok()
		.and_then(|s| s.parse().ok())
		.unwrap_or(level);


	let sub = tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::from_default_env()
				.add_directive(level.into()),
		) // ensure logs dont go to stdout which is used by the mcp server
		.with_writer(std::io::stderr)
		// dont format the output
		.with_ansi(false);
	#[cfg(debug_assertions)]
	// remove timestamps from the output in debug mode
	let sub = sub.without_time();
	sub.init();
}
