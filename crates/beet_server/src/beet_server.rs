use std::path::PathBuf;

use crate::prelude::*;
use anyhow::Result;
use axum::Router;



/// The main server struct for Beet.
/// By default a file server will be used as a fallback.
pub struct BeetServer {
	pub html_dir: PathBuf,
	pub no_file_server: bool,
	pub router: Router,
}

impl Default for BeetServer {
	fn default() -> Self {
		Self {
			html_dir: "target".into(),
			router: Router::new().merge(state_utils_routes()),
			no_file_server: false,
		}
	}
}

impl BeetServer {
	pub async fn serve(mut self) -> Result<()> {
		if !self.no_file_server {
			self.router = self
				.router
				.fallback_service(file_and_error_handler(&self.html_dir));
		}
		#[cfg(feature = "lambda")]
		return run_lambda(self.router).await;
		#[cfg(not(feature = "lambda"))]
		return run_axum(self.router).await;
	}
}
