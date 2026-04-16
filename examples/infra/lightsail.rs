//! Lightsail instance example with full deploy lifecycle.
//!
//! Supports the full CLI: validate, plan, apply, deploy,
//! rollback, rollforward, show, list, destroy.
//!
//! ```sh
//! cargo run --example lightsail --features=lightsail_block -- validate
//! cargo run --example lightsail --features=lightsail_block -- plan
//! cargo run --example lightsail --features=lightsail_block -- apply
//! cargo run --example lightsail --features=lightsail_block -- deploy
//! cargo run --example lightsail --features=lightsail_block -- show
//! cargo run --example lightsail --features=lightsail_block -- destroy --force
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
		Stack::new("lightsail-example").with_backend(LocalBackend::default()),
		LightsailBlock::default(),
		// cargo zigbuild for Lightsail: glibc-compatible Linux binary
		CargoBuildCmd::default()
			.cmd("zigbuild")
			.release()
			.example("router")
			.target("x86_64-unknown-linux-gnu.2.34")
			.feature("http_server"),
		stack_cli(),
		// deploy: build, apply infra, SCP binary to instance, restart service
		OnSpawn::insert_child(route(
			"deploy",
			(exchange_sequence(), children![
				CargoBuildAction,
				DeployLightsailAction,
			]),
		)),
	));
}
