//! # Hello Lambda
//!
//! Deploys the router example as a Lambda function.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_lambda --features=deploy,lambda_block -- validate
//! cargo run --example hello_lambda --features=deploy,lambda_block -- plan
//! cargo run --example hello_lambda --features=deploy,lambda_block -- deploy
//! cargo run --example hello_lambda --features=deploy,lambda_block -- show
//! cargo run --example hello_lambda --features=deploy,lambda_block -- destroy --force
//! ```

#[path = "../router/utils.rs"]
mod utils;
use beet::prelude::*;
use utils::*;

fn main() -> AppExit {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin {
				level: Level::TRACE,
				..default()
			},
			RouterAppPlugin,
			InfraPlugin,
		))
		.add_systems(Startup, setup)
		.run()
}

fn setup(mut commands: Commands) -> Result {
	cfg_if! {
		if #[cfg(feature="deploy")]{
			commands.spawn(infra_scene()?);
		}else{
			commands.spawn(router_scene()?);
		}
	}
	Ok(())
}

#[cfg(feature = "deploy")]
fn infra_scene() -> Result<impl Bundle> {
	(stack(), stack_cli(), children![route(
		"deploy",
		(exchange_sequence(), children![
			(
				LambdaBlock::default(),
				CargoBuild::default()
					.with_release(true)
					.with_example("hello_lambda")
					.with_additional_args(vec![
						"--features".into(),
						"http_server,lambda,router,infra,aws_sdk,bindings_aws_common".into(),
					])
					.into_lambda_build_artifact()
			),
			TofuApplyAction,
			(SyncS3BucketAction, assets_bucket_block()),
		]),
	)])
		.xok()
}
