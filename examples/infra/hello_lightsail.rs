//! # Hello Lightsail
//!
//! Deploys the router example as a Lightsail instance.
//! Assets are uploaded to S3 during deploy and accessed at runtime
//! via aws_sdk, identical to the Lambda pattern.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- validate
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- plan
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- deploy
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- watch
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- show
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- destroy --force
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
			RouterPlugin,
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
				Bucket::new(assets_bucket()),
				router::router_scene()?
			));
		}
	}
	Ok(())
}

#[cfg(feature = "deploy")]
fn infra_scene() -> Result<impl Bundle> {
	let block = LightsailBlock::default();
	(stack(), stack_cli(), assets_s3_fs_bucket(), children![
		route(
			"watch",
			(exchange_sequence(), children![
				AwsWatch::for_lightsail(&stack(), &block),
			])
		),
		route(
			"deploy",
			(exchange_sequence(), children![
				(
					block.clone(),
					CargoBuild::default()
						.with_release(true)
						.with_target(BuildTarget::Zigbuild)
						.with_example("hello_lightsail")
						.with_additional_args(vec![
							"--features".into(),
							"http_server,router,aws_sdk,bindings_aws_common"
								.into(),
						])
						.into_build_artifact()
				),
				TofuApplyAction,
				SyncS3BucketAction,
				AwsWatch::for_lightsail(&stack(), &block)
					.with_timeout(Duration::from_secs(30)),
			]),
		),
	])
		.xok()
}

/// The stack is used by both infra and router for resolving bucket names.
#[allow(unused)]
fn stack() -> Stack {
	Stack::new("hello_lightsail").with_aws_region("us-west-2")
}

#[cfg(feature = "bindings_aws_common")]
fn assets_bucket_block() -> S3BucketBlock {
	S3BucketBlock::new("assets").with_deploy_versioned(true)
}

#[cfg(feature = "deploy")]
fn assets_s3_fs_bucket() -> S3FsBucket {
	let stk = stack();
	S3FsBucket::new(
		FsBucket::new(WsPathBuf::new("examples/assets")),
		assets_bucket_block().provider(&stk),
	)
}

/// Resolve the assets bucket. Identical to the Lambda pattern:
/// on deployed instances, assets are accessed via S3 at runtime.
/// During local development, assets are read from the workspace.
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
