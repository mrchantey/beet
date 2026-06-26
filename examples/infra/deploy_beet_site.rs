//! Deploy the beet website to AWS Fargate (us-west-2): html over http and a live
//! TUI over ssh, the whole site (`main.bsx`, `routes/`, `templates/`, `assets/`)
//! served from S3, autoscaling between `min_count` and `max_count`.
//!
//! The container runs the `beet` binary as `beet serve --server=http,ssh`. The
//! binary pulls the site from the S3 site bucket at boot (`BEET_SERVICE_ACCESS=remote`
//! + `BEET_SITE_BUCKET`), so a site change re-publishes by re-syncing the bucket
//! with no image rebuild. A single NLB carries http on 80 (`/health` is the
//! health check), ssh on 22, and https on 443 (ACM cert, DNS-validated through
//! Cloudflare), and a cpu target-tracking policy autoscales the service.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk,cloudflare_dns -- validate
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk,cloudflare_dns -- plan
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk,cloudflare_dns -- deploy
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk,cloudflare_dns -- sync
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk,cloudflare_dns -- watch
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk,cloudflare_dns -- destroy --force
//! ```
//!
//! Requires the AWS CLI, opentofu, a container engine (podman/docker), and zig
//! (zigbuild), plus AWS credentials in `.env`.

use beet::prelude::*;

fn main() -> AppExit {
	// `.env` supplies the AWS creds and the stable `BEET_SSH_HOST_KEY`; drop any
	// `AWS_PROFILE` so every subprocess (tofu, the aws CLI, the S3 sync) uses the
	// explicit `.env` keys rather than a global profile.
	env_ext::load_dotenv();
	unsafe { env_ext::remove_var("AWS_PROFILE") };

	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin {
				level: Level::INFO,
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
		if #[cfg(feature = "deploy")] {
			commands
				.spawn(infra_scene()?)
				.trigger(StartRunning::boot);
		} else {
			let _ = &mut commands;
			bevybail!("the deploy_beet_site example requires the `fargate_block` feature");
		}
	}
	Ok(())
}

/// The stack namespaces every aws resource; `dev` stage (the default) deploys the
/// dev subdomain.
#[cfg(feature = "deploy")]
fn stack() -> Stack { Stack::new("beet-site").with_aws_region("us-west-2") }

/// The private site bucket (`main.bsx`, `routes/`, `templates/`), read by the
/// task's IAM role. Non-versioned so `beet sync` overwrites in place (live reload)
/// and the running task reads a stable root: `./site` syncs to the bucket root.
#[cfg(feature = "deploy")]
fn site_bucket() -> S3BucketBlock {
	S3BucketBlock::new("site").with_deploy_versioned(false)
}

/// The public-read assets bucket (`./assets`: images, favicon). Public-read so the
/// asset route's 301 to the S3 object URL resolves; non-versioned so `./assets`
/// syncs to a stable bucket root the container reads via `BEET_ASSETS_BUCKET`.
#[cfg(feature = "deploy")]
fn assets_bucket() -> S3BucketBlock {
	S3BucketBlock::new("assets")
		.with_deploy_versioned(false)
		.with_public_read(true)
}

/// The infra scene: the standard IaC commands plus `deploy` (build → tofu →
/// image → sync → watch), `sync` (re-publish the site without an image rebuild),
/// and `watch` (poll the rollout).
#[cfg(feature = "deploy")]
fn infra_scene() -> Result<impl Bundle> {
	let stk = stack();
	// the container reconstructs the two stores from these bucket names, plus
	// `BEET_SERVICE_ACCESS`/`AWS_REGION`/`BEET_DEPLOY_ID` the block already injects.
	let site_bucket_name = site_bucket().store(&stk).bucket_name().to_string();
	let assets_bucket_name =
		assets_bucket().store(&stk).bucket_name().to_string();
	// the stable ssh host key, so all 1..N tasks present one fingerprint.
	let ssh_host_key = env_ext::var("BEET_SSH_HOST_KEY").unwrap_or_default();
	// the Cloudflare zone the dns + ACM-validation records are published into.
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();

	// ssh + dns/https on one NLB; autoscales on cpu between min/max_count. 1 vCPU
	// / 2 GB: the default 0.25 vCPU is too small, even a paced idle server sits
	// ~80% CPU on it, pinning above the 50% scale target so it never scales in.
	// At 1 vCPU an idle task is ~20%, leaving real headroom for the load ramp.
	// Every hostname is DNS-only (not proxied) so it reaches the NLB directly for
	// raw-TCP ssh and so the cert terminates at the origin; a single SAN cert
	// covers them all. `beet.org`/`www.beet.org` (the apex prod hostnames) point
	// here too, DNS-only bypasses the proxied `beet.org -> beetstack.dev` redirect
	// rule, which is left intact, as is `beetstack.dev`.
	let block = FargateBlock::default()
		.with_allow_ssh(true)
		.with_dns(DnsProvider::cloudflare("dev.beet.org", zone_id.clone()))
		.with_dns(DnsProvider::cloudflare("beet.org", zone_id.clone()))
		.with_dns(DnsProvider::cloudflare("www.beet.org", zone_id))
		.with_max_count(5)
		.with_cpu(1024)
		.with_memory(2048)
		.with_static_env("BEET_SERVICE_ACCESS", "remote")
		.with_static_env("BEET_SITE_BUCKET", site_bucket_name)
		.with_static_env("BEET_ASSETS_BUCKET", assets_bucket_name)
		.with_static_env("BEET_SSH_HOST_KEY", ssh_host_key);

	// a `CliServer` host carrying the standard IaC verbs (validate/plan/apply/
	// show/list/destroy) plus this site's `deploy`/`sync`/`watch` commands, in
	// one `children!` so the host has a single `Children` relation.
	(stack(), CliServer::default(), default_router(), children![
		Validate,
		Plan,
		Apply,
		Show,
		List,
		Destroy,
		route(
			"watch",
			(ExchangeSequence, children![AwsWatch::for_fargate(
				&stack(),
				&block
			)])
		),
		route(
			"deploy",
			(ExchangeSequence, children![
				block.clone(),
				// create the site + assets buckets (and the ECR repo) via terraform.
				site_bucket(),
				assets_bucket(),
				// the `beet` binary to containerize: serve with http + ssh + S3.
				build_beet_binary(),
				// infrastructure first (creates the ECR repo the image pushes to,
				// plus the bucket, VPC, ALB/NLB and ECS service).
				TofuApplyAction,
				// build + push the image now the ECR repo exists; the container runs
				// `beet serve --server=http,ssh --path=/` so the ssh TUI opens on the
				// home route (otherwise the `serve` positional becomes the route).
				BuildDockerImage::default().with_cmd_args([
					"serve",
					"--server=http,ssh",
					"--path=/",
				]),
				// publish the site, then the assets, to the one bucket.
				sync_site(&stk),
				sync_assets(&stk),
				// watch the service roll out the new task (allow for the
				// crash-loop-then-converge window: image pull + first site sync).
				AwsWatch::for_fargate(&stack(), &block)
					.with_timeout(Duration::from_secs(300)),
			])
		),
		route(
			"sync",
			(ExchangeSequence, children![
				sync_site(&stk),
				sync_assets(&stk),
			])
		),
	])
		.xok()
}

/// Build the `beet` binary (release, zigbuild) with the http + ssh servers and the
/// S3-backed store loading the deployed task reads its site through.
#[cfg(feature = "deploy")]
fn build_beet_binary() -> impl Bundle {
	CargoBuild::default()
		.with_release(true)
		.with_target(BuildTarget::Zigbuild)
		.with_package("beet-cli")
		.with_binary("beet")
		.with_additional_args(vec!["--features".into(), "aws_sdk".into()])
		.into_build_artifact()
}

/// Sync `./site` to the bucket root (`main.bsx`, `routes/`, `templates/`).
#[cfg(feature = "deploy")]
fn sync_site(stack: &Stack) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("site")),
			site_bucket().store(stack),
		),
		SyncS3BucketAction,
	)
}

/// Sync `./assets` to the dedicated public-read assets bucket root (images,
/// favicon), which the container reads via `BEET_ASSETS_BUCKET`.
#[cfg(feature = "deploy")]
fn sync_assets(stack: &Stack) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("assets")),
			assets_bucket().store(stack),
		),
		SyncS3BucketAction,
	)
}
