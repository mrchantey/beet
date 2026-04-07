//! Lambda + API Gateway + Cloudflare DNS example using the typed provider API.
//!
//! Run with:
//! ```sh
//!   cargo run --example lambda --features=lambda_block
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	App::new()
		.spawn((Stack::default_local(), children![LambdaBlock::default()]));

	// let stack = ;
	// let lambda = ;
	// let config = lambda.build_config(&stack)?;

	let out_path =
		WsPathBuf::new("target/examples/lambda/main.tf.json").into_abs();
	config.export_and_validate(&out_path).await?;
	Ok(())
}
