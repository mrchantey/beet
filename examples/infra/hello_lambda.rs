//! # Hello Lambda
//!
//! Deploys `examples/bsx_site` as an http-only Lambda function (behind an API
//! Gateway URL, no custom domain). The generic `beet` binary is packaged as a
//! `provided.al2023` function and reads the site from an S3 bucket at request
//! time (`BEET_SERVICE_ACCESS=remote` + `BEET_SITE_BUCKET`), so a site change
//! re-publishes by re-running `sync` with no redeploy.
//!
//! Lambda is invocation-scoped: each request runs in a (possibly reused)
//! execution environment, sharing the warm-world-reuse concern with Cloudflare
//! Workers (see `hello_cloudflare_workers`). The shared shape lives in
//! `utils.rs`; this one differs only in its block (`LambdaBlock`), its build
//! (the lambda bootstrap zip), and its watch (`AwsWatch::for_lambda`).
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_lambda --features=router,lambda_block,markdown -- validate
//! cargo run --example hello_lambda --features=router,lambda_block,markdown -- deploy
//! cargo run --example hello_lambda --features=router,lambda_block,markdown -- sync
//! cargo run --example hello_lambda --features=router,lambda_block,markdown -- watch
//! cargo run --example hello_lambda --features=router,lambda_block,markdown -- destroy --force
//! ```

#[path = "utils.rs"]
mod utils;
use beet::prelude::*;
use utils::*;

fn main() -> AppExit { deploy_main(infra_scene) }

fn infra_scene() -> Result<impl Bundle> {
	let stk = stack("hello_lambda");
	let block =
		LambdaBlock::default().with_env_vars(remote_env(site_bucket_name(&stk)));

	(
		stack("hello_lambda"),
		stack_cli(),
		deploy_route(
			block.clone(),
			build_beet_lambda_binary("lambda,aws_sdk"),
			&stk,
			AwsWatch::for_lambda(&stk, &block)
				.with_timeout(Duration::from_secs(30)),
		),
		sync_route(&stk),
		watch_route(AwsWatch::for_lambda(&stk, &block)),
	)
		.xok()
}
