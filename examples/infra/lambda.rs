//! Lambda + API Gateway example with full deploy lifecycle.
//!
//! Supports the full CLI: validate, plan, apply, deploy,
//! rollback, rollforward, show, list, destroy.
//!
//! ```sh
//! cargo run --example lambda --features=lambda_block -- validate
//! cargo run --example lambda --features=lambda_block -- plan
//! cargo run --example lambda --features=lambda_block -- deploy
//! cargo run --example lambda --features=lambda_block -- show
//! cargo run --example lambda --features=lambda_block -- destroy --force
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
		// cargo lambda handles cross-compilation for Lambda's AL2023 runtime
		CargoBuildCmd::default()
			.cmd("lambda build")
			.release()
			.example("router")
			.feature("http_server")
			.feature("lambda"),
		stack_cli(),
		// deploy: build, package as lambda.zip, apply infrastructure
		OnSpawn::insert_child(route(
			"deploy",
			(exchange_sequence(), children![
				CargoBuildAction,
				PackageLambdaAction,
				TofuApplyAction,
			]),
		)),
	));
}
