//! Deploy the `ssh_tui_site` server (html over http + a multi-tenant live
//! terminal over ssh) to AWS Fargate, with autoscaling and an ssh-capable load
//! balancer (see [`FargateBlock`]).
//!
//! The container runs the `ssh_tui_site` example binary, which reads
//! `BEET_HOST=0.0.0.0` / `BEET_PORT` / `BEET_SSH_PORT` (set by [`FargateBlock`])
//! and serves both protocols. The ALB carries http (path `/health` is the health
//! check), an NLB carries ssh, and an autoscaling policy tracks cpu.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --example deploy_ssh_site --features=router,fargate_block,markdown -- validate
//! cargo run --example deploy_ssh_site --features=router,fargate_block,markdown -- plan
//! cargo run --example deploy_ssh_site --features=router,fargate_block,markdown -- deploy
//! cargo run --example deploy_ssh_site --features=router,fargate_block,markdown -- show
//! cargo run --example deploy_ssh_site --features=router,fargate_block,markdown -- destroy --force
//! ```
//!
//! Requires the AWS CLI, opentofu, and a container engine (podman/docker) on the
//! deploying machine, plus AWS credentials. The `deploy` route builds the static
//! binary (zig), applies the terraform (creating the ECR repo, VPC, ALB+NLB, ECS
//! service and autoscaling), builds and pushes the image, then watches rollout.

use beet::prelude::*;

fn main() -> AppExit {
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
			commands.spawn(infra_scene()?).trigger(StartServer::all);
		} else {
			let _ = &mut commands;
			bevybail!("the deploy_ssh_site example requires the `fargate_block` feature");
		}
	}
	Ok(())
}

/// The stack namespaces every aws resource; pick your region.
#[cfg(feature = "deploy")]
fn stack() -> Stack {
	Stack::new("beet-ssh-site").with_aws_region("us-west-2")
}

/// The infra scene: the standard IaC commands plus a `deploy` route that builds
/// the container binary, applies the terraform, pushes the image and watches it.
#[cfg(feature = "deploy")]
fn infra_scene() -> Result<impl Bundle> {
	// the default is http-only, so opt into ssh (an NLB on the ssh port). the
	// block also autoscales on cpu between min_count and max_count; the http
	// health check is `/health`.
	let block = FargateBlock::default().with_allow_ssh(true);
	(stack(), stack_cli(), children![
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
				// the static binary to containerize: the ssh_tui_site server.
				build_site_binary(),
				// infrastructure first (creates the ECR repo the image pushes to).
				TofuApplyAction,
				// build + push the image now the ECR repo exists.
				BuildDockerImageAction,
				// watch the service roll out the new task.
				AwsWatch::for_fargate(&stack(), &block)
					.with_timeout(Duration::from_secs(60)),
			])
		),
	])
	.xok()
}

/// Build the `ssh_tui_site` example as a release binary for the container, with
/// the http + multi-tenant ssh server features.
#[cfg(feature = "deploy")]
fn build_site_binary() -> impl Bundle {
	CargoBuild::default()
		.with_release(true)
		.with_target(BuildTarget::Zigbuild)
		.with_example("ssh_tui_site")
		.with_additional_args(vec![
			"--features".into(),
			"ssh_tui,http_server,markdown".into(),
		])
		.into_build_artifact()
}
