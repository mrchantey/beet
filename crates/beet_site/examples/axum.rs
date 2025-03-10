use beet::prelude::*;

#[rustfmt::skip]
#[tokio::main]
async fn main() {
	BeetServer{
		public_dir: "target/client".into(),
	}
		.serve_axum()
		.await
		.unwrap();
}
