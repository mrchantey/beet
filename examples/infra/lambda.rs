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
		.add_plugins((MinimalPlugins, InfraPlugin, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
	Ok(())
}


fn setup(mut commands: Commands) {
	commands.spawn((
		Stack::new("lambda-example").with_backend(LocalBackend::default()),
		LambdaBlock::default(),
		stack_cli(),
	));
}
