use crate::prelude::*;
use anyhow::Result;
use axum::Router;




pub struct BeetServer {
	pub public_dir: String,
	/// Serve via cargo-lambda instead of axum
	pub lambda: bool,
}

impl Default for BeetServer {
	fn default() -> Self {
		Self {
			public_dir: "target".into(),
			lambda: false,
		}
	}
}

impl BeetServer {
	pub fn router(&self) -> Router {
		Router::new()
			.merge(state_utils_routes())
			.fallback_service(file_and_error_handler(&self.public_dir))
	}

	pub async fn serve(&self) -> Result<()> {
		let router = self.router();
		if self.lambda {
			#[cfg(feature = "lambda")]
			return run_lambda(router)
				.await
				.map_err(|err| anyhow::anyhow!("{}", err));
			#[cfg(not(feature = "lambda"))]
			anyhow::bail!("Feature 'lambda' is not enabled");
		} else {
			return run_axum(router).await;
		}
	}
}
