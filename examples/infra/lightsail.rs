//! Typed Lightsail infrastructure example.
//!
//! Run with:
//! ```sh
//!   cargo run --example lightsail --features=lightsail_block,bindings_aws_common,aws
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
		Stack::new("lightsail-example"),
		stack_cli(),
		LightsailBlock::default(),
	));
}
