mod axum_ext;
mod json_query_param;
mod layers;
pub use axum_ext::*;
pub use json_query_param::*;
pub use layers::*;
mod state;
pub use state::*;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// tracing for local development, lambda has its own thing
pub fn init_axum_tracing() {
	tracing_subscriber::registry()
		.with(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| {
					// axum logs rejections from built-in extractors with the `axum::rejection`
					// target, at `TRACE` level. `axum::rejection=trace` enables showing those events
					format!(
						"{}=debug,tower_http=debug,axum::rejection=trace",
						env!("CARGO_CRATE_NAME")
					)
					.into()
				}),
		)
		.with(tracing_subscriber::fmt::layer())
		.init();
}
