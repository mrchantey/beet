//! Typed Lightsail infrastructure example.
//!
//! Run with:
//! ```sh
//!   cargo run --example lightsail --features=lightsail_block
//! ```

use beet::prelude::*;
#[beet::main]
async fn main() -> Result {
	App::new()
		.add_plugins((MinimalPlugins, BeetRouterPlugin, LogPlugin {
			// level: Level::TRACE,
			..default()
		}))
		.add_systems(Startup, setup)
		.run();
	Ok(())
}

fn setup(mut commands: Commands) {
	commands.spawn((
		Stack::new("lambda-example"),
		CliServer::default(),
		stack_router(),
		LightsailBlock::default(),
	));
}
