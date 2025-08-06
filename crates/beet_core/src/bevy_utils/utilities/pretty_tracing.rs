use bevy::log::tracing;
use bevy::log::tracing_subscriber;
use std::env;
use std::str::FromStr;
use tracing::level_filters::LevelFilter;

/// Opinionated high level tracing initialization
pub struct PrettyTracing {
	default_level: tracing::Level,
}

impl Default for PrettyTracing {
	fn default() -> Self {
		#[cfg(test)]
		let default_level = tracing::Level::WARN;
		#[cfg(not(test))]
		let default_level = tracing::Level::DEBUG;
		Self { default_level }
	}
}

impl PrettyTracing {
	/// Opinionated tracing defaults for bevy, using the provided level
	/// if none in environment variables.
	/// if already initialized, this will do nothing
	///
	/// ## AWS Lambda
	/// This also considers the AWS Lambda tracing environment variables,
	/// as defined in [`lambda_http::tracing::init_default_subscriber_with_writer`]
	///
	pub fn init(&self) {
		let log_level = env::var("AWS_LAMBDA_LOG_LEVEL")
			.or_else(|_| env::var("RUST_LOG"))
			.map(|val| LevelFilter::from_str(&val).ok())
			.ok()
			.flatten()
			.unwrap_or(self.default_level.into());

		let sub = tracing_subscriber::fmt()
			.compact()
			.with_level(true)
			.with_target(false)
			.with_thread_ids(false)
			.with_thread_names(false)
			.with_file(true)
			.without_time()
			.with_line_number(true)
			.with_env_filter(
				tracing_subscriber::EnvFilter::from_default_env()
					.add_directive("tower_http=debug".parse().unwrap())
					.add_directive("axum::rejection=trace".parse().unwrap())
					.add_directive("wgpu=error".parse().unwrap())
					.add_directive("naga=warn".parse().unwrap())
					.add_directive("bevy_app=warn".parse().unwrap())
					.add_directive("walrus=warn".parse().unwrap())
					.add_directive(log_level.into()),
			)
			.with_writer(std::io::stdout);
		// #[cfg(debug_assertions)]
		// // remove timestamps from the output in debug mode
		// let sub = sub.without_time();


		// if self.aws_lambda && std::env::var("AWS_LAMBDA_LOG_FORMAT")
		// 	.unwrap_or_default()
		// 	.eq_ignore_ascii_case("json") {
		// 	sub.json().try_init()
		// }else{
		// 	sub.pretty().try_init()
		// }.ok();
		sub.pretty().try_init().ok();
	}
}
