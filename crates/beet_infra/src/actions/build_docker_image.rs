//! Docker/Podman build and push action for Fargate deployments.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Container engine to use for building images.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect,
)]
pub enum ContainerEngine {
	#[default]
	/// Podman engine.
	Podman,
	/// Docker engine.
	Docker,
}

impl ContainerEngine {
	/// Get the command name for this engine.
	fn command(&self) -> &'static str {
		match self {
			Self::Docker => "docker",
			Self::Podman => "podman",
		}
	}
}

/// Builds and pushes a container image to ECR for Fargate deployment.
/// Looks for a [`BuildArtifact`] sibling to find the binary to containerize,
/// and a [`FargateBlock`] sibling for the ECR repository name and the `CMD`
/// bootstrap config.
///
/// The image `CMD` is not configured here: it is rendered from the sibling
/// [`FargateBlock`]'s [`BootstrapConfig`], so the deployed binary's argv and the
/// task definition's env come from one declaration.
#[action]
#[derive(Debug, Default, Component, Serialize, Deserialize, Reflect)]
#[reflect(Component, Default)]
pub async fn BuildDockerImage(
	/// Container engine to use (Docker or Podman).
	#[field]
	engine: ContainerEngine,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	// the resolved stack (region, resource names) and this launch's deploy id
	let (stack, deployment) = cx
		.caller
		.with_state::<StackQuery, _>(|entity, query| {
			(query.resolve(entity), query.deployment())
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
					.map(|child_of| child_of.parent())
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
		.await??;

	let binary_path = AbsPath::new(artifact.artifact_path())?;
	if !fs_ext::exists(&binary_path)? {
		bevybail!("binary not found at: {}", binary_path);
	}

	info!("building docker image for binary: {}", binary_path);

	// setup dockerfile directory
	let workspace_root = AbsPath::new_workspace_rel(".")?;
	let dockerfile_dir = workspace_root
		.join("target")
		.join(format!("{}-docker", artifact.label()));
	fs_ext::create_dir_all(&dockerfile_dir)?;

	// copy binary to dockerfile directory
	let binary_filename = binary_path
		.file_name()
		.ok_or_else(|| bevyhow!("invalid binary path"))?;
	let binary_dest = dockerfile_dir.join(binary_filename);
	fs_ext::copy(&binary_path, &binary_dest)?;

	// create Dockerfile
	let base_image = block.container_image().base_image();
	// add ca-certificates for Debian-based images
	let setup_commands = if base_image.contains("debian") {
		"RUN apt-get update && apt-get install -y ca-certificates && rm -rf \
			/var/lib/apt/lists/*\n"
	} else {
		""
	};

	// expose the http port, and the ssh port only when `allow_ssh`. EXPOSE is
	// documentation; the load balancers + security group make ports reachable
	// (see `FargateBlock`), so this mirrors the terraform ssh gating.
	let mut expose = format!("EXPOSE {}\n", block.container_port());
	if block.allow_ssh() {
		expose.push_str(&format!("EXPOSE {}\n", block.ssh_container_port()));
	}
	// the `CMD` runs the copied binary at `/app` with the block's argv-channel
	// bootstrap config and the route it boots, eg `["/app", "--home=s3://<bucket>",
	// "--server=http,ssh", "serve"]`, so deploy config reaches the binary as argv
	// (the site itself is pulled from S3 at boot, not baked in). Rendered as a
	// real JSON array, so the encoding is correct by construction rather than by
	// a hand-written quote join.
	let cmd_json = block
		.cmd_bootstrap()
		.to_cmd_json("/app", block.exec_route())?;
	let dockerfile_content = format!(
		"FROM {}\n{}COPY {} /app\nRUN chmod +x /app\n{}CMD {}\n",
		base_image, setup_commands, binary_filename, expose, cmd_json,
	);
	fs_ext::write(dockerfile_dir.join("Dockerfile"), dockerfile_content)?;

	// the ECR repository terraform names, and a login to its registry
	let ecr_repo_name = stack
		.resource_ident(block.build_label("ecr"))
		.primary_identifier()
		.to_string();
	let ecr = EcrRegistry::new(AwsClient::new(
		AwsService::ECR,
		stack.aws_region()?.clone(),
	));
	let ecr_url = ecr.repository_uri(&ecr_repo_name).await?;
	let engine_cmd = engine.command();
	info!("authenticating {engine_cmd} to ECR");
	let login = ecr.login().await?;
	login
		.command(engine)
		.run_async_stdin(&login.password)
		.await
		.map_err(|err| bevyhow!("ECR authentication failed: {err}"))?;

	// build container image
	let image_tag = format!("{}:{}", ecr_url, deployment.deploy_id());
	info!("building {} image: {image_tag}", engine_cmd);
	let build_cmd = ChildProcess::new(engine_cmd).with_args([
		"build",
		"-t",
		&image_tag,
		"--platform",
		"linux/amd64",
		dockerfile_dir.as_str(),
	]);
	build_cmd
		.run_async()
		.await
		.map_err(|err| bevyhow!("{} build failed: {err}", engine_cmd))?;

	// push to ECR
	info!("pushing {} image to ECR", engine_cmd);
	let push_cmd =
		ChildProcess::new(engine_cmd).with_args(["push", &image_tag]);
	push_cmd
		.run_async()
		.await
		.map_err(|err| bevyhow!("{} push failed: {err}", engine_cmd))?;

	info!("{} image pushed: {image_tag}", engine_cmd);
	Pass(cx.input).xok()
}

/// The ECR registry of the client's account and region.
struct EcrRegistry {
	client: AwsClient,
}

/// A registry login: the host and the password `<engine> login` takes on
/// stdin, never in argv.
struct EcrLogin {
	registry: String,
	password: String,
}

impl EcrRegistry {
	fn new(client: AwsClient) -> Self { Self { client } }

	/// The push uri of the repository `name`, ie
	/// `<account>.dkr.ecr.<region>.amazonaws.com/<name>`.
	///
	/// The repository is terraform's, created by the `storage` layer: an
	/// absent one is refused rather than created, since a repository made here
	/// collides with the one the next apply creates.
	async fn repository_uri(&self, name: &str) -> Result<String> {
		let answer = self
			.client
			.call_json(
				"DescribeRepositories",
				&value!({ "repositoryNames": [name] }),
			)
			.await;
		match answer {
			Ok(answer) => answer["repositories"][0]["repositoryUri"]
				.as_str()
				.ok()
				.map(ToString::to_string)
				.ok_or_else(|| {
					bevyhow!("ECR answered no uri for repository {name}")
				}),
			Err(err)
				if AwsError::is(&err, &["RepositoryNotFoundException"]) =>
			{
				bevybail!(
					"ECR repository {name} does not exist. Run Terraform apply \
					 first to create infrastructure."
				)
			}
			Err(err) => Err(err),
		}
	}

	/// `ecr:GetAuthorizationToken`, its `AWS:<password>` token decoded.
	async fn login(&self) -> Result<EcrLogin> {
		let answer = self
			.client
			.call_json("GetAuthorizationToken", &value!({}))
			.await?;
		let data = &answer["authorizationData"][0];
		let registry = data["proxyEndpoint"]
			.as_str()
			.ok()
			.ok_or_else(|| bevyhow!("ECR answered no registry endpoint"))?
			.trim_start_matches("https://")
			.to_string();
		let token = data["authorizationToken"]
			.as_str()
			.ok()
			.ok_or_else(|| bevyhow!("ECR answered no authorization token"))?;
		let password = Self::decode_token(token)?
			.strip_prefix("AWS:")
			.ok_or_else(|| {
				bevyhow!("ECR's authorization token names another user")
			})?
			.to_string();
		EcrLogin { registry, password }.xok()
	}

	fn decode_token(token: &str) -> Result<String> {
		cfg_if! {
			if #[cfg(feature = "deploy")] {
				use base64::Engine;
				base64::engine::general_purpose::STANDARD
					.decode(token)?
					.xmap(String::from_utf8)?
					.xok()
			} else {
				let _ = token;
				bevybail!("decoding an ECR login requires the `deploy` feature")
			}
		}
	}
}

impl EcrLogin {
	/// `<engine> login` to the registry, reading the password from stdin and
	/// redacting it from every rendering of the command.
	fn command(&self, engine: ContainerEngine) -> ChildProcess {
		ChildProcess::new(engine.command())
			.with_secret(self.password.as_str())
			.with_args([
				"login",
				"--username",
				"AWS",
				"--password-stdin",
				self.registry.as_str(),
			])
	}
}

impl BuildDockerImage {
	/// Create with Docker engine.
	pub fn with_docker() -> Self {
		Self {
			engine: ContainerEngine::Docker,
		}
	}

	/// Create with Podman engine.
	pub fn with_podman() -> Self {
		Self {
			engine: ContainerEngine::Podman,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// The registry whose ECR answers each call with `answer`, given its
	/// target and body, recording every call.
	fn registry(
		answer: impl 'static + Send + Sync + Fn(&str, &Value) -> Response,
	) -> (EcrRegistry, Arc<Mutex<Vec<(String, Value)>>>) {
		let calls = Arc::new(Mutex::new(Vec::new()));
		let recorded = calls.clone();
		let answer = Arc::new(answer);
		let client = AwsClient::new(AwsService::ECR, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| {
				let (recorded, answer) = (recorded.clone(), answer.clone());
				Box::pin(async move {
					request.url().to_string().xpect_eq(
						"https://api.ecr.ap-southeast-2.amazonaws.com/",
					);
					let target = request
						.headers
						.first_raw("x-amz-target")
						.unwrap_or_default()
						.to_string();
					let body = json_ext::from_slice::<Value>(
						&request.body.into_bytes().await?,
					)?;
					let response = answer(&target, &body);
					recorded.lock().unwrap().push((target, body));
					response.xok()
				})
			});
		(EcrRegistry::new(client), calls)
	}

	const REGISTRY: &str = "123456789012.dkr.ecr.ap-southeast-2.amazonaws.com";

	/// The repository's uri is ECR's own, and an absent repository is refused
	/// as terraform's to create.
	#[beet_core::test]
	async fn finds_the_repository() {
		let (ecr, calls) = registry(|_, body| {
			match body["repositoryNames"][0].as_str().ok() {
				Some("beetmash-prod--ecr") => Response::ok().with_body(
					value!({ "repositories": [{
						"repositoryName": "beetmash-prod--ecr",
						"repositoryUri": format!("{REGISTRY}/beetmash-prod--ecr"),
						"registryId": "123456789012",
					}]})
					.to_json_string(),
				),
				_ => Response::from_status(StatusCode::BAD_REQUEST).with_body(
					value!({
						"__type": "RepositoryNotFoundException",
						"message": "The repository does not exist",
					})
					.to_json_string(),
				),
			}
		});
		ecr.repository_uri("beetmash-prod--ecr")
			.await
			.unwrap()
			.xpect_eq(format!("{REGISTRY}/beetmash-prod--ecr"));
		ecr.repository_uri("beetmash-drill--ecr")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Run Terraform apply");
		calls.lock().unwrap()[0].clone().xpect_eq((
			"AmazonEC2ContainerRegistry_V20150921.DescribeRepositories"
				.to_string(),
			value!({ "repositoryNames": ["beetmash-prod--ecr"] }),
		));
	}

	/// The token decodes to the password the engine reads from stdin: the
	/// command is exactly the login with no password in its argv, and the
	/// password is redacted wherever it would print.
	#[beet_core::test]
	async fn logs_in_over_stdin() {
		// base64 of `AWS:hunter2`
		let (ecr, calls) = registry(|_, _| {
			Response::ok().with_body(
				value!({ "authorizationData": [{
					"authorizationToken": "QVdTOmh1bnRlcjI=",
					"expiresAt": 1_800_000_000.0,
					"proxyEndpoint": format!("https://{REGISTRY}"),
				}]})
				.to_json_string(),
			)
		});
		let login = ecr.login().await.unwrap();
		login.password.as_str().xpect_eq("hunter2");
		login
			.command(ContainerEngine::Podman)
			.to_string()
			.xpect_eq(format!(
				"podman login --username AWS --password-stdin {REGISTRY}"
			));
		login
			.command(ContainerEngine::Podman)
			.with_args(["echo", "hunter2"])
			.to_string()
			.xnot()
			.xpect_contains("hunter2");
		calls.lock().unwrap()[0].clone().xpect_eq((
			"AmazonEC2ContainerRegistry_V20150921.GetAuthorizationToken"
				.to_string(),
			value!({}),
		));
	}
}
