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
		.add_plugins((
			MinimalPlugins,
			BeetRouterPlugin,
			// LogPlugin {
			// 	level: Level::TRACE,
			// 	..default()
			// },
		))
		.add_systems(Startup, setup)
		.run();

	// let out_path =
	// 	WsPathBuf::new("target/examples/lambda/main.tf.json").into_abs();
	// config.export_and_validate(&out_path).await?;
	Ok(())
}


fn setup(mut commands: Commands) {
	commands.spawn((
		Stack::new("lambda-example"),
		CliServer::default(),
		stack_router(),
		LambdaBlock::default(),
	));
}
