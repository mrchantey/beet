//! Lambda + API Gateway + Cloudflare DNS example using the typed provider API.
//!
//! Run with:
//! ```sh
//!   cargo run --example lambda --features=fs,rand,stack_lambda
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let cx = StackContext::default();
	let stack = Stack::new(LocalBackend::default());

	let lambda = LambdaStack::default();
	let config = lambda.build_config(&cx, &stack);

	let out_path = WsPathBuf::new("target/examples/lambda/main.tf.json");
	config.export_and_validate(&out_path).await?;
	Ok(())
}
