use crate::prelude::*;
use anyhow::Result;
use axum::Router;




pub struct BeetServer {
	pub public_dir: String,
	/// Serve via cargo-lambda instead of axum
	pub lambda: bool,
	pub no_file_server: bool,
	pub router: Router,
}

impl Default for BeetServer {
	fn default() -> Self {
		Self {
			public_dir: "target".into(),
			router: Router::new().merge(state_utils_routes()),
			lambda: false,
			no_file_server: false,
		}
	}
}

impl BeetServer {
	pub async fn serve(mut self) -> Result<()> {
		if !self.no_file_server {
			self.router = self
				.router
				.fallback_service(file_and_error_handler(&self.public_dir));
		}

		if self.lambda {
			#[cfg(feature = "lambda")]
			return run_lambda(self.router)
				.await
				.map_err(|err| anyhow::anyhow!("{}", err));
			#[cfg(not(feature = "lambda"))]
			anyhow::bail!("Feature 'lambda' is not enabled");
		} else {
			return run_axum(self.router).await;
		}
	}
}
