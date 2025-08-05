use bevy::log::tracing;
use bevy::log::tracing_subscriber;
use bevy::log::tracing_subscriber::EnvFilter;


/// Opinionated tracing defaults for bevy,
/// if already initialized, this will do nothing
pub fn init_pretty_tracing(level: tracing::Level) {
	let sub = tracing_subscriber::fmt()
		.compact()
		.with_level(true)
		.with_target(false)
		.with_thread_ids(false)
		.with_thread_names(false)
		.with_file(true)
		.pretty()
		.with_line_number(true)
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| {
					EnvFilter::builder().parse_lossy(&format!(
						"tower_http=debug,axum::rejection=trace,wgpu=error,naga=warn,bevy_app=warn",
						// "{}=debug,tower_http=debug,axum::rejection=trace,wgpu=error,naga=warn,bevy_app=warn",
						// env!("CARGO_CRATE_NAME")
					))
				})
				.add_directive(level.into()),
		);
	#[cfg(debug_assertions)]
	// remove timestamps from the output in debug mode
	let sub = sub.without_time();
	sub.try_init().ok();
}
