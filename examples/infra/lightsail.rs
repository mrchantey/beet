//! Typed Lightsail infrastructure example.
//!
//! Run with:
//! ```sh
//!   cargo run --example lightsail --features=stack_lightsail
//! ```
use beet::prelude::*;

#[beet::main]
async fn main() -> Result {
	let cx = StackContext::default();
	let stack = Stack::new(LocalBackend::default());

	let lightsail = LightsailStack::default();
	let config = lightsail.build_config(&cx, &stack);

	let out_path =
		WsPathBuf::new("target/examples/lambda/main.tf.json").into_abs();
	config.export_and_validate(&out_path).await?;
	Ok(())
}
