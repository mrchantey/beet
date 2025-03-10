use crate::prelude::*;
use anyhow::Result;
use axum::Router;




pub struct BeetServer {
	pub public_dir: String,
}

impl Default for BeetServer {
	fn default() -> Self {
		Self {
			public_dir: "target".into(),
		}
	}
}

impl BeetServer {
	pub fn router(&self) -> Router {
		Router::new()
			.merge(state_utils_routes())
			.fallback_service(file_and_error_handler(&self.public_dir))
	}

	pub async fn serve_axum(&self) -> Result<()> {
		run_axum(self.router()).await
	}
	#[cfg(feature = "lambda")]
	pub async fn serve_lambda(&self) -> Result<(), lambda_http::Error> {
		run_lambda(self.router()).await
	}
}
