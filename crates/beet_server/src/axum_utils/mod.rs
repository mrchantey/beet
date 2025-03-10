use anyhow::Result;
use axum::Router;

mod axum_ext;
pub use axum_ext::*;
mod layers;
pub use layers::*;
mod state;
pub use state::*;



pub fn default_router_base() -> Router {
	Router::new()
		.merge(state_utils_routes())
		.fallback_service(file_and_error_handler("target"))
}



/// Runs the axum router indefinitely.
/// If the router contains an app state ensure it is initialized
/// using `.with_state()` before passing it to this function.
pub async fn run_axum(router: Router) -> Result<()> {
	#[cfg(feature = "lambda")]
	lambda_http::tracing::init_default_subscriber();
	let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
	#[cfg(feature = "lambda")]
	lambda_http::tracing::info!(
		"listening on http://{}",
		listener.local_addr()?,
	);

	axum::serve(listener, router).await?;
	Ok(())
}
