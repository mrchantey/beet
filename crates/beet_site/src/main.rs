#[allow(unused)]
use beet::prelude::*;
#[allow(unused)]
use beet_site::prelude::*;

/// The main entry point for beet site.
/// There are three options:
#[tokio::main]
async fn main() {
	// 1. build static site
	#[cfg(all(not(feature = "axum"), not(feature = "lambda")))]
	build_static().await.unwrap();
	// 2. build server locally in debug mode
	#[cfg(feature = "axum")]
	BeetServer {
		public_dir: "target/client".into(),
	}
	.serve_axum()
	.await
	.unwrap();
	// 3. build server for lambda in release mode
	#[cfg(all(not(feature = "axum"), feature = "lambda"))]
	BeetServer {
		public_dir: "target/client".into(),
	}
	.serve_lambda()
	.await
	.unwrap();
}
