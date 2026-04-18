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

#[path = "../router/router.rs"]
mod router;
use beet::prelude::*;


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
			commands.spawn((
				// make assets bucket accessible to routes
				Bucket::new(assets_bucket()),
				router::router_scene()?
			));
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
			(
				SyncS3Bucket::new("examples/assets"),
				// we also declare assets bucket here for sync
				assets_bucket_block()
			),
		]),
	)])
		.xok()
}



/// The stack is used by both infra and router
/// for resolving bucket names.
#[allow(unused)]
fn stack() -> Stack { Stack::new("hello_lambda").with_aws_region("us-west-2") }

#[cfg(feature = "bindings_aws_common")]
fn assets_bucket_block() -> S3BucketBlock {
	S3BucketBlock::new("assets").with_deploy_versioned(true)
}

#[allow(unused)]
fn assets_bucket() -> impl BucketProvider {
	cfg_if! {
		if #[cfg(all(feature = "aws_sdk", feature = "bindings_aws_common"))]{
			let stk = stack();
			assets_bucket_block().provider(&stk)
		}else{
			FsBucket::new(WsPathBuf::new("examples/assets"))
		}
	}
}
