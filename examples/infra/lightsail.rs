//! Typed Lightsail infrastructure example.
//!
//! Run with:
//! ```sh
//!   cargo run --example lightsail --features=lightsail_block
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let stack = Stack::default_local();
	let lightsail = LightsailBlock::default();
	let config = lightsail.build_config(&stack)?;

	let out_path =
		WsPathBuf::new("target/examples/lambda/main.tf.json").into_abs();
	config.export_and_validate(&out_path).await?;
	Ok(())
}
