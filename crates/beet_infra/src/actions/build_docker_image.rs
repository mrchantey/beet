//! Docker build and push action for Fargate deployments.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Builds and pushes a Docker image to ECR for Fargate deployment.
/// Looks for a [`BuildArtifact`] sibling to find the binary to containerize,
/// and a [`FargateBlock`] sibling to determine the ECR repository name.
#[action]
#[derive(Default, Component)]
pub async fn BuildDockerImageAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// get the stack for region and deploy_id
	let stack = cx
		.caller
		.with_state::<AncestorQuery<&Stack>, _>(|entity, query| {
			query.get(entity).map(|s| s.clone())
		})
		.await?;

	// get siblings by querying the parent's children
	let (block, artifact) = cx
		.caller
		.with_state::<(
			Query<&Children>,
			Query<&ChildOf>,
			Query<&FargateBlock>,
			Query<&BuildArtifact>,
		), _>(
			|entity,
			 (children_q, child_of_q, block_q, artifact_q)|
			 -> Result<_> {
				// find parent
				let parent = child_of_q
					.get(entity)
					.ok()
					.and_then(|child_of| Some(child_of.parent()))
					.ok_or_else(|| bevyhow!("no parent found"))?;

				// find siblings
				let children = children_q
					.get(parent)
					.map_err(|_| bevyhow!("parent has no children"))?;

				let mut block_opt = None;
				let mut artifact_opt = None;

				for child in children.iter() {
					if let Ok(b) = block_q.get(child) {
						block_opt = Some(b.clone());
					}
					if let Ok(a) = artifact_q.get(child) {
						artifact_opt = Some(a.clone());
					}
				}

				let block = block_opt.ok_or_else(|| {
					bevyhow!("no FargateBlock found in siblings")
				})?;
				let artifact = artifact_opt.ok_or_else(|| {
					bevyhow!("no BuildArtifact found in siblings")
				})?;

				(block, artifact).xok()
			},
		)
		.await?;

	let binary_path = AbsPathBuf::new(artifact.artifact_path())?;
	if !binary_path.exists() {
		bevybail!("binary not found at: {}", binary_path.display());
	}

	info!(
		"building docker image for binary: {}",
		binary_path.display()
	);

	// setup dockerfile directory
	let workspace_root = AbsPathBuf::new_workspace_rel(".")?;
	let dockerfile_dir = workspace_root
		.join("target")
		.join(format!("{}-docker", artifact.label()));
	std::fs::create_dir_all(&dockerfile_dir)?;

	// copy binary to dockerfile directory
	let binary_filename = binary_path
		.file_name()
		.ok_or_else(|| bevyhow!("invalid binary path"))?;
	let binary_dest = dockerfile_dir.join(binary_filename);
	std::fs::copy(&binary_path, &binary_dest)?;

	// create Dockerfile
	let base_image = block.container_image().base_image();
	// add ca-certificates for Debian-based images
	let setup_commands = if base_image.contains("debian") {
		"RUN apt-get update && apt-get install -y ca-certificates && rm -rf \
			/var/lib/apt/lists/*\n"
	} else {
		""
	};

	let dockerfile_content = format!(
		"FROM {}\n{}COPY {} /app\nRUN chmod +x /app\nEXPOSE {}\nCMD \
			[\"/app\"]\n",
		base_image,
		setup_commands,
		binary_filename.to_string_lossy(),
		block.container_port()
	);
	std::fs::write(dockerfile_dir.join("Dockerfile"), dockerfile_content)?;

	// get ECR repository details - must match terraform's naming convention
	// terraform uses stack.resource_ident(block.build_label("ecr")).primary_identifier()
	let ecr_ident = stack.resource_ident(block.build_label("ecr"));
	let ecr_repo_name = ecr_ident.primary_identifier();
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
			 --username AWS --password-stdin \
			 {account_id}.dkr.ecr.{region}.amazonaws.com"
		),
	]);
	auth_cmd
		.run_async()
		.await
		.map_err(|err| bevyhow!("ECR authentication failed: {err}"))?;

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
	build_cmd
		.run_async()
		.await
		.map_err(|err| bevyhow!("docker build failed: {err}"))?;

	// push to ECR
	info!("pushing docker image to ECR");
	let push_cmd = ChildProcess::new("docker").with_args(["push", &image_tag]);
	push_cmd
		.run_async()
		.await
		.map_err(|err| bevyhow!("docker push failed: {err}"))?;

	info!("docker image pushed: {image_tag}");
	Pass(cx.input).xok()
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

	create_cmd
		.run_async()
		.await
		.map_err(|err| bevyhow!("failed to create ECR repository: {err}"))?;

	Ok(())
}
