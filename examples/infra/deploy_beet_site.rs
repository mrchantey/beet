//! Deploy the beet website to AWS Fargate (us-west-2): html over http and a live
//! TUI over ssh, the whole site (`main.bsx`, `routes/`, `templates/`, `assets/`)
//! served from S3, autoscaling between `min_count` and `max_count`.
//!
//! The container runs the `beet` binary as `beet serve --server=http,ssh`. The
//! binary pulls the site from the S3 site bucket at boot (`BEET_SERVICE_ACCESS=remote`
//! + `BEET_SITE_BUCKET`), so a site change re-publishes by re-syncing the bucket
//! with no image rebuild. The ALB carries http (`/health` is the health check), an
//! NLB carries ssh, and a cpu target-tracking policy autoscales the service.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk -- validate
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk -- plan
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk -- deploy
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk -- sync
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk -- watch
//! cargo run --example deploy_beet_site --features=router,fargate_block,markdown,aws_sdk -- destroy --force
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
				.trigger(ActionIn::boot);
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

/// The single S3 bucket the site is served from. Non-versioned so `beet sync`
/// overwrites in place (live reload) and the running task reads a stable root:
/// `./site` syncs to the bucket root, `./assets` to its `assets/` prefix.
#[cfg(feature = "deploy")]
fn site_bucket() -> S3BucketBlock {
	S3BucketBlock::new("site").with_deploy_versioned(false)
}

/// The infra scene: the standard IaC commands plus `deploy` (build → tofu →
/// image → sync → watch), `sync` (re-publish the site without an image rebuild),
/// and `watch` (poll the rollout).
#[cfg(feature = "deploy")]
fn infra_scene() -> Result<impl Bundle> {
	let stk = stack();
	// the container reconstructs the site store from these: the bucket name, plus
	// `BEET_SERVICE_ACCESS`/`AWS_REGION`/`BEET_DEPLOY_ID` the block already injects.
	let site_bucket_name = site_bucket().store(&stk).bucket_name().to_string();
	// the stable ssh host key, so all 1..N tasks present one fingerprint.
	let ssh_host_key = env_ext::var("BEET_SSH_HOST_KEY").unwrap_or_default();

	// ssh + a domain (opens the 443 hole); autoscales on cpu between min/max_count.
	// 1 vCPU / 2 GB: the default 0.25 vCPU is too small — even a paced idle server
	// sits ~80% CPU on it, pinning above the 50% scale target so it never scales in.
	// At 1 vCPU an idle task is ~20%, leaving real headroom for the load ramp.
	let block = FargateBlock::default()
		.with_allow_ssh(true)
		.with_domain("dev.beet.org")
		.with_max_count(5)
		.with_cpu(1024)
		.with_memory(2048)
		.with_static_env("BEET_SERVICE_ACCESS", "remote")
		.with_static_env("BEET_SITE_BUCKET", site_bucket_name)
		.with_static_env("BEET_SSH_HOST_KEY", ssh_host_key);

	// a `CliServer` host carrying this site's `deploy`/`sync`/`watch` commands, in
	// one `children!` so the host has a single `Children` relation.
	(stack(), CliServer::default(), default_router(), children![
		route(
			"watch",
			(exchange_sequence(), children![AwsWatch::for_fargate(
				&stack(),
				&block
			)])
		),
		route(
			"deploy",
			(exchange_sequence(), children![
				block.clone(),
				// create the site bucket (and the ECR repo) via terraform.
				site_bucket(),
				// the `beet` binary to containerize: serve with http + ssh + S3.
				build_beet_binary(),
				// infrastructure first (creates the ECR repo the image pushes to,
				// plus the bucket, VPC, ALB/NLB and ECS service).
				TofuApplyAction,
				// build + push the image now the ECR repo exists; the container runs
				// `beet serve --server=http,ssh --path=/` so the ssh TUI opens on the
				// home route (otherwise the `serve` positional becomes the route).
				(
					BuildDockerImage::default().with_cmd_args([
						"serve",
						"--server=http,ssh",
						"--path=/",
					]),
					BuildDockerImageAction,
				),
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
			(exchange_sequence(), children![
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

/// Sync `./assets` to the bucket's `assets/` prefix (images, favicon).
#[cfg(feature = "deploy")]
fn sync_assets(stack: &Stack) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("assets")),
			site_bucket()
				.store(stack)
				.with_subdir(SmolPath::new("assets")),
		),
		SyncS3BucketAction,
	)
}
