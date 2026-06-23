//! # Hello Fargate
//!
//! Deploys `examples/bsx_site` as an http-only Fargate service. The generic
//! `beet` binary runs in the container (`beet serve --server=http`) and reads the
//! site from an S3 bucket at request time (`BEET_SERVICE_ACCESS=remote` +
//! `BEET_SITE_BUCKET`), so a site change re-publishes by re-running `sync` with
//! no image rebuild. The ALB serves http on its own DNS name (no custom domain).
//!
//! This is `deploy_beet_site` minus the ssh / domain / autoscaling showcase, and
//! the shared shape of every infra example (see `utils.rs`). Switching to
//! Cloudflare Containers is a block + deploy-route swap (see
//! `hello_cloudflare_containers`).
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_fargate --features=router,fargate_block,markdown -- validate
//! cargo run --example hello_fargate --features=router,fargate_block,markdown -- deploy
//! cargo run --example hello_fargate --features=router,fargate_block,markdown -- sync
//! cargo run --example hello_fargate --features=router,fargate_block,markdown -- watch
//! cargo run --example hello_fargate --features=router,fargate_block,markdown -- destroy --force
//! ```

#[path = "utils.rs"]
mod utils;
use beet::prelude::*;
use utils::*;

fn main() -> AppExit { deploy_main(infra_scene) }

fn infra_scene() -> Result<impl Bundle> {
	let stk = stack("hello_fargate");
	// the container reconstructs the site store from these.
	let block = FargateBlock::default()
		.with_env_vars(remote_env(site_bucket_name(&stk)));

	// `stack_cli()` carries the IaC verbs; the custom routes append via
	// `OnSpawn::insert_child` (a second `children!` would clobber them).
	(
		stack("hello_fargate"),
		stack_cli(),
		// containerized deploy: build the binary, apply the infra (creates the ECR
		// repo + bucket), build + push the image, then publish the site and watch.
		// block + build stay separate children so `BuildDockerImageAction` finds both.
		OnSpawn::insert_child(route(
			"deploy",
			(exchange_sequence(), children![
				block.clone(),
				site_bucket(),
				build_beet_binary("aws_sdk"),
				TofuApplyAction,
				(
					BuildDockerImage::default().with_cmd_args([
						"serve",
						"--server=http",
						"--path=/",
					]),
					BuildDockerImageAction,
				),
				sync_site(&stk),
				AwsWatch::for_fargate(&stk, &block)
					.with_timeout(Duration::from_secs(300)),
			]),
		)),
		sync_route(&stk),
		watch_route(AwsWatch::for_fargate(&stk, &block)),
	)
		.xok()
}
