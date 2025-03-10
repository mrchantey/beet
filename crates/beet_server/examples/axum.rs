use beet_server::prelude::*;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
	BeetServer::default()
		.serve_axum()
		.await
		.unwrap();
}
