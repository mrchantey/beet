//! # Hello Lightsail
//!
//! Deploys `examples/bsx_site` as an http-only Lightsail instance. The generic
//! `beet` binary runs under systemd and reads the site from an S3 bucket at
//! request time (`BEET_SERVICE_ACCESS=remote` + `BEET_SITE_BUCKET`), so a site
//! change re-publishes by re-running `sync` with no redeploy. Served on the
//! instance's public address (no custom domain).
//!
//! The shared shape of every infra example lives in `utils.rs`; this one differs
//! only in its block (`LightsailBlock`) and watch (`AwsWatch::for_lightsail`).
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_lightsail --features=router,lightsail_block,markdown -- validate
//! cargo run --example hello_lightsail --features=router,lightsail_block,markdown -- deploy
//! cargo run --example hello_lightsail --features=router,lightsail_block,markdown -- sync
//! cargo run --example hello_lightsail --features=router,lightsail_block,markdown -- watch
//! cargo run --example hello_lightsail --features=router,lightsail_block,markdown -- destroy --force
//! ```

#[path = "utils.rs"]
mod utils;
use beet::prelude::*;
use utils::*;

fn main() -> AppExit { deploy_main(infra_scene) }

fn infra_scene() -> Result<impl Bundle> {
	let stk = stack("hello_lightsail");
	let block = LightsailBlock::default()
		.with_env_vars(remote_env(site_bucket_name(&stk)));

	(
		stack("hello_lightsail"),
		stack_cli(),
		deploy_route(
			block.clone(),
			build_beet_binary("aws_sdk"),
			&stk,
			AwsWatch::for_lightsail(&stk, &block)
				.with_timeout(Duration::from_secs(30)),
		),
		sync_route(&stk),
		watch_route(AwsWatch::for_lightsail(&stk, &block)),
	)
		.xok()
}
