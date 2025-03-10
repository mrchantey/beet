use beet_server::prelude::*;

#[rustfmt::skip]
#[tokio::main]
async fn main() { 
	run_axum(default_router_base())
	.await
	.unwrap();
}
