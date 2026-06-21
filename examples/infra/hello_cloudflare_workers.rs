//! # Hello Cloudflare Workers
//!
//! Deploys `examples/bsx_site` to a Cloudflare Worker, where the Worker IS
//! `beet-cli` compiled to wasm (`--features cloudflare`) — one binary for every
//! target, native on Fargate/Lightsail/Lambda/Containers, wasm here. The Worker
//! reads the site from R2 through the native `worker::Bucket` binding (no S3
//! credentials at runtime), renders per request, and serves on
//! `<name>.workers.dev`.
//!
//! `wrangler deploy` runs `worker-build` (wasm-bindgen + wasm-opt) from the
//! `wrangler.jsonc` `build.command`, so there is no separate binary-build step;
//! the deploy is otherwise the same deploy/sync/watch/destroy shape as the
//! container example, with `CloudflareWorkerBlock` +
//! `CloudflareWorkerDeployAction` in place of the container block + action.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example hello_cloudflare_workers --features=router,cloudflare_block,markdown -- deploy
//! cargo run --example hello_cloudflare_workers --features=router,cloudflare_block,markdown -- sync
//! cargo run --example hello_cloudflare_workers --features=router,cloudflare_block,markdown -- watch
//! cargo run --example hello_cloudflare_workers --features=router,cloudflare_block,markdown -- destroy
//! ```
//!
//! Live deploy needs `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID` in `.env`,
//! plus `wrangler` and `worker-build`. No R2 keys are needed (native binding).

#[path = "utils.rs"]
mod utils;
use beet::prelude::*;
use utils::*;

fn main() -> AppExit { deploy_main(infra_scene) }

fn infra_scene() -> Result<impl Bundle> {
	let block = CloudflareWorkerBlock::new("beet-hello-worker")
		.with_bucket("beet-hello-site");
	let name = block.name().clone();
	let bucket = block.bucket().clone();

	(CliServer::default(), default_router(), children![
		route(
			"deploy",
			(exchange_sequence(), children![
				block.clone(),
				// wrangler runs worker-build (wasm) per the wrangler.jsonc build.command.
				CloudflareWorkerDeployAction,
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
			(exchange_sequence(), children![CloudflareDestroy::new(
				name.clone(),
				bucket.clone()
			)])
		),
	])
		.xok()
}
