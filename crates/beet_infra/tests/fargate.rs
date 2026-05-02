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

#[beet_core::test(timeout_ms = 1_800_000)]
// #[ignore = "deploys resources and takes ten minutes"]
async fn fargate_lifecycle() {
	pretty_env_logger::init();
	info!("==== STARTING FARGATE LIFECYCLE TEST ====");

	// resolve source paths and create revert guards
	let (source, _source_guard, assets_file, _assets_guard) =
		setup_source_guards(SOURCE_PATH, ASSETS_FILE).unwrap();

	let mut stack = Stack::new("fargate-test").with_aws_region("us-west-2");
	info!("Stack created: {}", stack.app_name());

	// clean up any prior state
	let project = build_project(&stack).unwrap();
	info!("About to cleanup_prior_state");
	cleanup_prior_state(&stack, project).await;
	info!("cleanup_prior_state complete");

	// 1. deploy v1
	info!("step 1: deploying v1");
	info!("About to call deploy()");
	deploy(&stack).await.unwrap();
	info!("deploy() completed successfully");

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
	info!("deploy: starting fargate deployment");
	info!("deploy: ENTRY POINT");
	let block = FargateBlock::default();
	let cargo = CargoBuild::default()
		.with_release(true)
		.with_target(BuildTarget::Zigbuild)
		.with_package("beet_infra")
		.with_example(EXAMPLE_NAME)
		.with_additional_args(vec!["--features".into(), "deploy".into()]);

	// STAGE 1: Run terraform to create ECR and basic infrastructure
	// This will create the ECR repository so we can push the Docker image
	info!("deploy: stage 1 - attempting terraform apply for ECR creation");
	let project = build_project(stack)?;
	let apply_result = project.apply().await;
	match &apply_result {
		Err(e) => {
			info!("deploy: terraform apply had errors: {}", e);
			info!("deploy: will manually ensure ECR exists");
		}
		Ok(_) => {
			info!("deploy: terraform apply succeeded");
		}
	}

	// Ensure ECR repository exists (either from terraform or create manually)
	let ecr_repo_name = "main-fargate";
	let region = stack.aws_region();
	ensure_ecr_exists(region, ecr_repo_name).await?;
	info!("deploy: ECR repository confirmed to exist");

	// STAGE 2: Build the binary
	info!("deploy: building binary with cargo");
	info!("deploy: stage 2 - building binary");
	let binary_path = build_binary_with_cargo(&cargo).await?;
	info!("binary built at: {}", binary_path.display());
	info!("deploy: binary built at: {}", binary_path.display());

	// STAGE 3: Build and push Docker image to ECR
	info!("deploy: building and pushing docker image");
	info!("deploy: stage 3 - building and pushing docker image");
	let ecr_url = build_and_push_image(stack, &binary_path).await?;
	info!("docker image pushed to: {ecr_url}");
	info!("deploy: docker image pushed to: {}", ecr_url);

	// STAGE 4: Run terraform again to complete the deployment with sync assets
	info!("deploy: stage 4 - final terraform apply with asset sync");
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
	info!("deploy: AsyncPlugin exchange completed");

	info!(
		"deploy: action sequence completed, response status: {}",
		response.status()
	);

	let status = response.status();
	if status.is_success() {
		info!("deploy complete - SUCCESS");
		Ok(())
	} else {
		let body = response.unwrap_str().await;
		bevybail!("deploy failed: {status} - {body}")
	}
}

/// Build the binary for the Fargate container using CargoBuild.
async fn build_binary_with_cargo(cargo: &CargoBuild) -> Result<AbsPathBuf> {
	info!("build_binary_with_cargo: starting");
	let artifact = cargo.clone().into_build_artifact();
	let cmd = artifact.process();

	info!("building binary: {}", cmd);
	let output = cmd.clone().run_async().await?;
	if !output.status.success() {
		bevybail!("build failed: {}", String::from_utf8_lossy(&output.stderr));
	}
	info!("build_binary_with_cargo: cargo build completed successfully");

	let binary_path = AbsPathBuf::new(artifact.artifact_path())?;

	if !binary_path.exists() {
		bevybail!("binary not found at: {}", binary_path.display());
	}
	info!(
		"build_binary_with_cargo: binary verified at {}",
		binary_path.display()
	);

	Ok(binary_path)
}

/// Build and push Docker image to ECR.
async fn build_and_push_image(
	stack: &Stack,
	binary_path: &AbsPathBuf,
) -> Result<String> {
	info!("build_and_push_image: starting");
	let workspace_root = AbsPathBuf::new_workspace_rel(".")?;
	let dockerfile_dir = workspace_root.join("target").join("fargate-test");
	info!(
		"build_and_push_image: creating dockerfile directory at {}",
		dockerfile_dir.display()
	);
	std::fs::create_dir_all(&dockerfile_dir)?;

	// copy binary to dockerfile directory
	info!("build_and_push_image: copying binary to dockerfile directory");
	let binary_dest = dockerfile_dir.join(EXAMPLE_NAME);
	std::fs::copy(binary_path, &binary_dest)?;

	// create simple Dockerfile using Debian to match glibc-linked binary
	info!("build_and_push_image: creating Dockerfile");
	let dockerfile_content = format!(
		"FROM debian:bookworm-slim\n\
		 COPY {EXAMPLE_NAME} /app\n\
		 RUN chmod +x /app\n\
		 EXPOSE {}\n\
		 CMD [\"/app\"]\n",
		beet_net::prelude::DEFAULT_SERVER_PORT
	);
	std::fs::write(dockerfile_dir.join("Dockerfile"), dockerfile_content)?;

	// get ECR repository URL
	info!("build_and_push_image: getting AWS account ID and ECR details");
	let ecr_repo_name = "main-fargate";
	let region = stack.aws_region();
	let account_id = get_aws_account_id().await?;
	let ecr_url =
		format!("{account_id}.dkr.ecr.{region}.amazonaws.com/{ecr_repo_name}");
	info!("build_and_push_image: ECR URL: {}", ecr_url);

	// ECR repository was created by terraform in stage 1

	// authenticate Docker to ECR
	info!("authenticating docker to ECR");
	let auth_cmd = ChildProcess::new("sh").with_args([
		"-c",
		&format!(
			"aws ecr get-login-password --region {region} | docker login \
			 --username AWS --password-stdin {account_id}.dkr.ecr.{region}.amazonaws.com"
		),
	]);
	info!("build_and_push_image: running ECR authentication");
	let output = auth_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"ECR auth failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}
	info!("build_and_push_image: ECR authentication successful");

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
	info!("build_and_push_image: running docker build");
	let output = build_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"docker build failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}
	info!("build_and_push_image: docker build successful");

	// push to ECR
	info!("pushing docker image to ECR");
	let push_cmd = ChildProcess::new("docker").with_args(["push", &image_tag]);
	info!("build_and_push_image: running docker push");
	let output = push_cmd.run_async().await?;
	if !output.status.success() {
		bevybail!(
			"docker push failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}
	info!("build_and_push_image: docker push successful");

	info!(
		"build_and_push_image: complete, returning ECR URL: {}",
		ecr_url
	);
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

/// Ensure ECR repository exists, creating it if necessary.
async fn ensure_ecr_exists(region: &str, repo_name: &str) -> Result {
	// check if repository exists
	let check_cmd = ChildProcess::new("aws").with_args([
		"ecr",
		"describe-repositories",
		"--region",
		region,
		"--repository-names",
		repo_name,
	]);

	let check_result = check_cmd.run_async().await;
	match check_result {
		Ok(output) if output.status.success() => {
			info!("ECR repository '{}' already exists", repo_name);
			return Ok(());
		}
		_ => {
			// repository doesn't exist, create it
		}
	}

	// create repository
	info!("Creating ECR repository: {}", repo_name);
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
			"Failed to create ECR repository: {}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	info!("Successfully created ECR repository: {}", repo_name);
	Ok(())
}

/// Fargate-specific verify_live wrapper.
/// Fargate serves via ALB DNS name on port 80.
async fn verify_live(dns: &str, expected: &str) -> Result {
	let url = format!("http://{dns}/version");
	runtime_utils::verify_live(&url, expected, 40, 15).await
}
