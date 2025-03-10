use beet_site::prelude::*;

/// The main entry point for a beet server
#[tokio::main]
async fn main() {
	build_static().await.unwrap();
}
