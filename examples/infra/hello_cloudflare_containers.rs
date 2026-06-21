//! # Hello Cloudflare Containers
//!
//! Deploys `examples/bsx_site` to Cloudflare Containers: the SAME native `beet`
//! binary as `hello_fargate`, run in a container that reads the site from R2 at
//! request time via [`S3Store::r2`]. Served on `<name>.workers.dev` (no custom
//! domain). Toggling between this and Fargate is a small swap:
//!
//! - block: `CloudflareContainerBlock` <-> `FargateBlock`
//! - deploy action: `CloudflareContainerDeployAction` (+ wrangler) <-> `TofuApplyAction` + `BuildDockerImageAction`
//! - sync: `CloudflareR2Sync` (R2) <-> `sync_site` (S3)
//! - watch: `CloudflareWatch` <-> `AwsWatch::for_fargate`
//!
//! The binary build (`build_beet_binary`), the served content, and the
//! deploy/sync/watch route shape are identical (see `utils.rs`).
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_cloudflare_containers --features=router,cloudflare_block,markdown -- deploy
//! cargo run --example hello_cloudflare_containers --features=router,cloudflare_block,markdown -- sync
//! cargo run --example hello_cloudflare_containers --features=router,cloudflare_block,markdown -- watch
//! cargo run --example hello_cloudflare_containers --features=router,cloudflare_block,markdown -- destroy
//! ```
//!
//! Live deploy needs `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`, and the R2
//! data-plane keys (`R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY`) in `.env`, plus
//! `wrangler` and a container engine. The Workers Paid plan is required for
//! Containers.

#[path = "utils.rs"]
mod utils;
use beet::prelude::*;
use utils::*;

fn main() -> AppExit { deploy_main(infra_scene) }

fn infra_scene() -> Result<impl Bundle> {
	let block = CloudflareContainerBlock::new("beet-hello-container")
		.with_bucket("beet-hello-site");
	let name = block.name().clone();
	let bucket = block.bucket().clone();

	// no `Stack`/`stack_cli` (Cloudflare is wrangler-provisioned, not terraform):
	// a `CliServer` host carrying the deploy/sync/watch/destroy commands.
	(CliServer::default(), default_router(), children![
		route(
			"deploy",
			(exchange_sequence(), children![
				block.clone(),
				build_beet_binary("aws_sdk"),
				CloudflareContainerDeployAction,
				CloudflareR2Sync::new("examples/bsx_site", bucket.clone()),
				CloudflareWatch::new(name.clone()),
			])
		),
		route(
			"sync",
			(exchange_sequence(), children![CloudflareR2Sync::new(
				"examples/bsx_site",
				bucket.clone()
			)])
		),
		route(
			"destroy",
			(exchange_sequence(), children![
				CloudflareDestroy::new(name.clone(), bucket.clone())
					// empty the synced site objects before deleting the bucket.
					.with_local_dir("examples/bsx_site")
			])
		),
	])
		.xok()
}
