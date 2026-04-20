use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;

const EXAMPLE_NAME: &str = "lightsail_test";
const SOURCE_PATH: &str = "examples/infra/lightsail_test.rs";
const MARKER_V1: &str = "test-v1";
const MARKER_V2: &str = "test-v2";

#[beet_core::test(timeout_ms = 900_000)]
#[ignore = "requires AWS infrastructure"]
async fn lightsail_lifecycle() {
	// resolve source path and create revert guard
	let source = AbsPathBuf::new_workspace_rel(SOURCE_PATH).unwrap();
	let original = std::fs::read_to_string(source.as_path()).unwrap();
	let _guard = SourceRevert {
		path: source.clone(),
		original,
	};

	let mut stack = Stack::new("lightsail-test").with_aws_region("us-west-2");

	// clean up any prior state
	let project = build_project(&stack).unwrap();
	project.force_destroy().await;
	let client = stack.artifacts_client();
	client.bucket().bucket_remove().await.ok();

	// 1. deploy v1
	info!("step 1: deploying v1");
	deploy(&stack).await.unwrap();

	// 2. get public address and verify v1
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 2: verifying v1 at {address}");
	verify_live(&address, MARKER_V1).await.unwrap();

	// 3-4. modify source to v2, deploy again
	info!("step 3-4: deploying v2");
	swap_version(&source, MARKER_V1, MARKER_V2).unwrap();
	stack = Stack::new("lightsail-test").with_aws_region("us-west-2");
	deploy(&stack).await.unwrap();

	// 5. verify v2
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 5: verifying v2 at {address}");
	verify_live(&address, MARKER_V2).await.unwrap();

	// 6. rollback to v1
	info!("step 6: rolling back");
	let client = stack.artifacts_client();
	client.rollback(1).await.unwrap();
	apply_with_current_ledger(&mut stack).await.unwrap();

	// 7. verify v1 after rollback
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 7: verifying v1 after rollback at {address}");
	verify_live(&address, MARKER_V1).await.unwrap();

	// 8. rollforward to v2
	info!("step 8: rolling forward");
	let client = stack.artifacts_client();
	client.rollforward().await.unwrap();
	apply_with_current_ledger(&mut stack).await.unwrap();

	// 9. verify v2 after rollforward
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 9: verifying v2 after rollforward at {address}");
	verify_live(&address, MARKER_V2).await.unwrap();

	// 10. destroy
	info!("step 10: destroying");
	let project = build_project(&stack).unwrap();
	project.destroy().await.unwrap();
	let client = stack.artifacts_client();
	client.bucket().bucket_remove().await.ok();

	// 11. verify dead
	info!("step 11: verifying dead");
	verify_dead(&address).await.unwrap();

	// revert source (guard also handles this)
	swap_version(&source, MARKER_V2, MARKER_V1).ok();
}


/// Guard that reverts source file changes on drop.
struct SourceRevert {
	path: AbsPathBuf,
	original: String,
}

impl Drop for SourceRevert {
	fn drop(&mut self) {
		std::fs::write(self.path.as_path(), &self.original).ok();
	}
}

/// Build the terraform project for the Lightsail test stack.
fn build_project(stack: &Stack) -> Result<terra::Project> {
	let block = LightsailBlock::default();
	let mut config = stack.create_config();
	config.add_provider_config(
		&terra::Provider::AWS,
		&serde_json::json!({ "region": stack.aws_region() }),
	)?;
	let mut world = World::new();
	block.apply_to_config(
		&world.spawn(()).as_readonly(),
		stack,
		&mut config,
	)?;
	terra::Project::new(stack, config).xok()
}

/// Build the lightsail binary and return the BuildArtifact.
fn lightsail_build_artifact() -> BuildArtifact {
	CargoBuild::default()
		.with_release(true)
		.with_target(BuildTarget::Zigbuild)
		.with_package("beet")
		.with_example(EXAMPLE_NAME)
		.with_additional_args(vec![
			"--features".into(),
			"http_server,router,infra".into(),
		])
		.into_build_artifact()
}

/// Build, upload artifacts, and apply terraform.
async fn deploy(stack: &Stack) -> Result<String> {
	// build binary
	let artifact = lightsail_build_artifact();
	info!("building lightsail binary");
	artifact.process().clone().run_async().await?;

	// upload artifact to S3
	let mut client = stack.artifacts_client();
	client.ensure_bucket().await?;
	let bytes = fs_ext::read_async(artifact.artifact_path()).await?;
	let source_hash = artifact.compute_source_hash()?;
	let label = LightsailBlock::default().label().clone();
	client
		.upload_artifact(&label, bytes, ArtifactEntry {
			bucket_key: stack.artifact_key(&label).into(),
			source_hash: source_hash.into(),
		})
		.await?;
	client.publish_ledger().await?;

	// apply terraform
	let project = build_project(stack)?;
	let output = project.apply().await?;
	info!("deploy complete");
	Ok(output)
}

/// Re-apply terraform with the current ledger deploy_id,
/// used after rollback/rollforward to update the instance.
async fn apply_with_current_ledger(stack: &mut Stack) -> Result<String> {
	let client = stack.artifacts_client();
	let ledger = client
		.current_ledger()
		.await?
		.ok_or_else(|| bevyhow!("no current ledger"))?;
	stack.update_from_ledger(&ledger);
	let project = build_project(stack)?;
	project.apply().await
}

/// Verify the deployed endpoint returns the expected version marker.
/// Lightsail instances serve on port 80 via public IP.
async fn verify_live(address: &str, expected: &str) -> Result {
	let url = format!("http://{address}/version");
	let mut last_err = bevyhow!("no attempts made");
	// lightsail instances take longer to boot than lambda
	for attempt in 0..30 {
		match Request::get(&url).send().await {
			Ok(res) if res.status().is_success() => {
				let body = res.text().await?;
				if body.contains(expected) {
					info!("verified: {expected} (attempt {attempt})");
					return Ok(());
				}
				last_err = bevyhow!("expected '{expected}' but got '{body}'");
			}
			Ok(res) => {
				last_err = bevyhow!("HTTP {}", res.status());
			}
			Err(err) => {
				last_err = err;
			}
		}
		time_ext::sleep(Duration::from_secs(10)).await;
	}
	Err(last_err)
}

/// Verify the endpoint is no longer reachable.
async fn verify_dead(address: &str) -> Result {
	let url = format!("http://{address}/version");
	time_ext::sleep(Duration::from_secs(10)).await;
	for _ in 0..5 {
		match Request::get(&url).send().await {
			Err(_) => return Ok(()),
			Ok(res) if !res.status().is_success() => return Ok(()),
			Ok(_) => {}
		}
		time_ext::sleep(Duration::from_secs(5)).await;
	}
	bevybail!("endpoint still reachable after destroy")
}

/// Modify the test binary source to use a different version marker.
fn swap_version(path: &AbsPathBuf, from: &str, to: &str) -> Result {
	let content = std::fs::read_to_string(path.as_path())?;
	let updated =
		content.replacen(&format!("\"{}\"", from), &format!("\"{}\"", to), 1);
	if content == updated {
		bevybail!("marker '{from}' not found in {}", path.display());
	}
	std::fs::write(path.as_path(), &updated)?;
	Ok(())
}
