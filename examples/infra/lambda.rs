//! Lambda + API Gateway + Cloudflare DNS example using the typed provider API.
//!
//! Run with:
//! ```sh
//!   cargo run --example lambda --features=lambda_block
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let stack = Stack::default_local();
	let lambda = LambdaBlock::default();
	let config = lambda.build_config(&stack);

	let out_path =
		WsPathBuf::new("target/examples/lambda/main.tf.json").into_abs();
	config.export_and_validate(&out_path).await?;
	Ok(())
}
