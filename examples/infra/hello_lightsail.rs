//! # Hello Lightsail
//!
//! Deploys the router example as a Lightsail instance.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- validate
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- plan
//! cargo run --example hello_lightsail --features=deploy,lightsail_block -- deploy
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
				LightsailBlock::default(),
				LightsailAssets::new("examples/assets"),
				CargoBuild::default()
					.with_example("hello_lightsail")
					.with_additional_args(vec![
						"--features".into(),
						"http_server,router,infra".into(),
					])
					.into_musl_build_artifact()
			),
			TofuApplyAction,
			DeployLightsailAction,
		]),
	)])
		.xok()
}

/// Resolve the assets bucket, preferring BEET_ASSETS_DIR on deployed instances.
#[allow(unused)]
fn assets_bucket() -> FsBucket {
	match env_ext::var("BEET_ASSETS_DIR") {
		Ok(dir) => FsBucket::new(AbsPathBuf::new(dir).unwrap()),
		Err(_) => FsBucket::new(WsPathBuf::new("examples/assets")),
	}
}

#[allow(unused)]
fn stack() -> Stack { Stack::new("hello_lightsail").with_aws_region("us-east-1") }
