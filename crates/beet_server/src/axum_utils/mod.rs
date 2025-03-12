mod axum_ext;
pub use axum_ext::*;
mod layers;
pub use layers::*;
mod state;
pub use state::*;

use anyhow::Result;
use axum::Router;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Sets up tracing and runs the axum server.
/// If the router contains an app state ensure it is initialized
/// using `.with_state()` before passing it to this function.
pub async fn run_axum(router: Router) -> Result<()> {
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
	let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
	tracing::info!("listening on http://{}", listener.local_addr()?);

	axum::serve(listener, router).await?;
	Ok(())
}
