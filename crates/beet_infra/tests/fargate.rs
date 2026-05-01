#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Integration test for Fargate Block.
//! Takes approx 10 mins.
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

mod runtime_utils;
use runtime_utils::*;

const EXAMPLE_NAME: &str = "fargate_test";
const SOURCE_PATH: &str = "crates/beet_infra/examples/fargate_test.rs";

#[beet_core::test(timeout_ms = 900_000)]
#[ignore = "deploys resources and takes ten minutes"]
async fn fargate_lifecycle() {
	pretty_env_logger::init();

	// resolve source paths and create revert guards
	let (source, _source_guard, assets_file, _assets_guard) =
		setup_source_guards(SOURCE_PATH, ASSETS_FILE).unwrap();

	let mut stack = Stack::new("fargate-test").with_aws_region("us-west-2");

	// clean up any prior state
	let project = build_project(&stack).unwrap();
	cleanup_prior_state(&stack, project).await;

	// 1. deploy v1
	info!("step 1: deploying v1");
	deploy(&stack).await.unwrap();

	// 2. verify v1 is live
	let project = build_project(&stack).unwrap();
	let url = project.output("load_balancer_dns").await.unwrap();
	info!("step 2: verifying v1 at {url}");
	verify_live(&url, MARKER_V1).await.unwrap();

	// 3. verify v1 assets
	info!("step 3: verifying v1 assets");
	verify_assets(&stack, MARKER_V1).await.unwrap();

	// 4-5. modify source and assets to v2, deploy again
	info!("step 4-5: deploying v2");
	swap_version(&source, MARKER_V1, MARKER_V2).unwrap();
	swap_version(&assets_file, MARKER_V1, MARKER_V2).unwrap();
	stack = Stack::new("fargate-test").with_aws_region("us-west-2");
	deploy(&stack).await.unwrap();

	// 6. verify v2
	let project = build_project(&stack).unwrap();
	let url = project.output("load_balancer_dns").await.unwrap();
	info!("step 6: verifying v2 at {url}");
	verify_live(&url, MARKER_V2).await.unwrap();

	// 7. verify v2 assets
	info!("step 7: verifying v2 assets");
	verify_assets(&stack, MARKER_V2).await.unwrap();

	// 8. rollback to v1
	info!("step 8: rolling back");
	let client = stack.artifacts_client();
	client.rollback(1).await.unwrap();
	apply_with_current_ledger(&mut stack, build_project)
		.await
		.unwrap();

	// 9. verify v1 after rollback
	let project = build_project(&stack).unwrap();
	let url = project.output("load_balancer_dns").await.unwrap();
	info!("step 9: verifying v1 after rollback at {url}");
	verify_live(&url, MARKER_V1).await.unwrap();

	// 10. verify v1 assets after rollback
	info!("step 10: verifying v1 assets after rollback");
	verify_assets(&stack, MARKER_V1).await.unwrap();

	// 11. rollforward to v2
	info!("step 11: rolling forward");
	let client = stack.artifacts_client();
	client.rollforward().await.unwrap();
	apply_with_current_ledger(&mut stack, build_project)
		.await
		.unwrap();

	// 12. verify v2 after rollforward
	let project = build_project(&stack).unwrap();
	let url = project.output("load_balancer_dns").await.unwrap();
	info!("step 12: verifying v2 after rollforward at {url}");
	verify_live(&url, MARKER_V2).await.unwrap();

	// 13. verify v2 assets after rollforward
	info!("step 13: verifying v2 assets after rollforward");
	verify_assets(&stack, MARKER_V2).await.unwrap();

	// 14. destroy
	info!("step 14: destroying");
	let project = build_project(&stack).unwrap();
	cleanup(&stack, project).await.unwrap();

	// 15. verify dead
	info!("step 15: verifying dead");
	verify_dead(&url, 30, 10, 10).await.unwrap();

	// revert source files (guards also handle this)
	swap_version(&source, MARKER_V2, MARKER_V1).ok();
	swap_version(&assets_file, MARKER_V2, MARKER_V1).ok();
}

/// Build the terraform project for the Fargate test stack.
fn build_project(stack: &Stack) -> Result<terra::Project> {
	let block = FargateBlock::default();
	let bucket_block = assets_bucket_block();
	let mut config = stack.create_config();
	config.add_provider_config(
		&terra::Provider::AWS,
		&serde_json::json!({ "region": stack.aws_region() }),
	)?;
	let mut world = World::new();
	let entity_mut = world.spawn(());
	let entity = entity_mut.as_readonly();
	block.apply_to_config(&entity, stack, &mut config)?;
	bucket_block.apply_to_config(&entity, stack, &mut config)?;
	terra::Project::new(stack, config).xok()
}

/// Build, upload artifacts, sync assets, build/push Docker image, and apply terraform.
async fn deploy(stack: &Stack) -> Result {
	// first, build the binary
	let binary_path = build_binary(stack).await?;
	info!("binary built at: {}", binary_path.display());

	// build and push Docker image to ECR
	let ecr_url = build_and_push_image(stack, &binary_path).await?;
	info!("docker image pushed to: {ecr_url}");

	// now deploy the infrastructure with assets
	let block = FargateBlock::default();
	let response = AsyncPlugin::world()
		.spawn((
			stack.clone(),
			assets_s3_fs_bucket(stack),
			assets_bucket_block(),
			exchange_sequence(),
			children![block, TofuApplyAction, SyncS3BucketAction,],
		))
		.exchange(Request::get(""))
		.await;

	let status = response.status();
	if status.is_success() {
		info!("deploy complete");
		Ok(())
	} else {
		let body = response.unwrap_str().await;
		bevybail!("deploy failed: {status} - {body}")
	}
}

/// Build the binary for the Fargate container.
async fn build_binary(_stack: &Stack) -> Result<AbsPathBuf> {
	let target_dir_str = env_ext::var("CARGO_TARGET_DIR")
		.unwrap_or_else(|_| "target".to_string());
	let target_dir = if target_dir_str.starts_with('/') {
		// absolute path
		AbsPathBuf::new(&target_dir_str)?
	} else {
		// relative to workspace
		AbsPathBuf::new_workspace_rel(&target_dir_str)?
	};
	let binary_name = EXAMPLE_NAME;

	// build using zigbuild for x86_64-unknown-linux-musl
	let cmd = ChildProcess::new("cargo").with_args([
		"zigbuild",
		"--release",
		"--target",
		"x86_64-unknown-linux-musl",
		"-p",
		"beet_infra",
		"--example",
		EXAMPLE_NAME,
		"--features",
		"deploy",
	]);

	info!("building binary: {}", cmd);
	let output = cmd.run_async().await?;
	if !output.status.success() {
		bevybail!("build failed: {}", String::from_utf8_lossy(&output.stderr));
	}

	let binary_path = target_dir
		.join("x86_64-unknown-linux-musl")
		.join("release")
		.join("examples")
		.join(binary_name);

	if !binary_path.exists() {
		bevybail!("binary not found at: {}", binary_path.display());
	}

	Ok(binary_path)
}

/// Build and push Docker image to ECR.
async fn build_and_push_image(
	stack: &Stack,
	binary_path: &AbsPathBuf,
) -> Result<String> {
	let workspace_root = AbsPathBuf::new_workspace_rel(".")?;
	let dockerfile_dir = workspace_root.join("target").join("fargate-test");
	std::fs::create_dir_all(&dockerfile_dir)?;

	// copy binary to dockerfile directory
	let binary_dest = dockerfile_dir.join(EXAMPLE_NAME);
	std::fs::copy(binary_path, &binary_dest)?;

	// create simple Dockerfile
	let dockerfile_content = format!(
		"FROM alpine:latest\n\
		 COPY {EXAMPLE_NAME} /app\n\
		 RUN chmod +x /app\n\
		 EXPOSE {}\n\
		 CMD [\"/app\"]\n",
		beet_net::prelude::DEFAULT_SERVER_PORT
	);
	std::fs::write(dockerfile_dir.join("Dockerfile"), dockerfile_content)?;

	// get ECR repository URL
	let ecr_repo_name = "main-fargate";
	let region = stack.aws_region();
	let account_id = get_aws_account_id().await?;
	let ecr_url =
		format!("{account_id}.dkr.ecr.{region}.amazonaws.com/{ecr_repo_name}");

	// ensure ECR repository exists
	ensure_ecr_repository(region, ecr_repo_name).await?;

	// authenticate Docker to ECR
	info!("authenticating docker to ECR");
	let auth_cmd = ChildProcess::new("sh").with_args([
		"-c",
		&format!(
			"aws ecr get-login-password --region {region} | docker login \
			 --username AWS --password-stdin {account_id}.dkr.ecr.{region}.amazonaws.com"
		),
	]);
	let output = auth_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"ECR auth failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	// build Docker image
	let image_tag = format!("{}:{}", ecr_url, stack.deploy_id());
	info!("building docker image: {image_tag}");
	let build_cmd = ChildProcess::new("docker").with_args([
		"build",
		"-t",
		&image_tag,
		"--platform",
		"linux/amd64",
		dockerfile_dir.to_str().unwrap(),
	]);
	let output = build_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"docker build failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	// push to ECR
	info!("pushing docker image to ECR");
	let push_cmd = ChildProcess::new("docker").with_args(["push", &image_tag]);
	let output = push_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"docker push failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	Ok(ecr_url)
}

/// Get AWS account ID.
async fn get_aws_account_id() -> Result<String> {
	let cmd = ChildProcess::new("aws").with_args([
		"sts",
		"get-caller-identity",
		"--query",
		"Account",
		"--output",
		"text",
	]);
	let output = cmd.run_async().await?;
	if !output.status.success() {
		bevybail!("failed to get AWS account ID");
	}
	String::from_utf8(output.stdout)
		.map_err(|e| bevyhow!("invalid UTF-8 in account ID: {e}"))
		.map(|s| s.trim().to_string())
}

/// Ensure ECR repository exists.
async fn ensure_ecr_repository(region: &str, repo_name: &str) -> Result {
	let cmd = ChildProcess::new("aws").with_args([
		"ecr",
		"describe-repositories",
		"--region",
		region,
		"--repository-names",
		repo_name,
	]);

	// check if repo exists (non-zero exit is ok, means it doesn't exist)
	let result = cmd.run_async().await;
	match result {
		Ok(output) if output.status.success() => {
			info!("ECR repository {repo_name} already exists");
			return Ok(());
		}
		_ => {
			// repo doesn't exist, create it
		}
	}

	// create repository
	info!("creating ECR repository: {repo_name}");
	let create_cmd = ChildProcess::new("aws").with_args([
		"ecr",
		"create-repository",
		"--region",
		region,
		"--repository-name",
		repo_name,
	]);

	let output = create_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"failed to create ECR repository: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	Ok(())
}

/// Fargate-specific verify_live wrapper.
/// Fargate serves via ALB DNS name on port 80.
async fn verify_live(dns: &str, expected: &str) -> Result {
	let url = format!("http://{dns}/version");
	runtime_utils::verify_live(&url, expected, 40, 15).await
}
