#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Integration test for Lightsail Block.
//! Takes approx 10 mins.
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

const EXAMPLE_NAME: &str = "lightsail_test";
const SOURCE_PATH: &str = "crates/beet_infra/examples/lightsail_test.rs";
const ASSETS_PATH: &str = "examples/infra/assets";
const ASSETS_FILE: &str = "examples/infra/assets/index.html";
const MARKER_V1: &str = "test-v1";
const MARKER_V2: &str = "test-v2";

#[beet_core::test(timeout_ms = 900_000)]
#[ignore = "deploys resources and takes ten minutes"]
async fn lightsail_lifecycle() {
	pretty_env_logger::init();

	// resolve source paths and create revert guards
	let source = AbsPathBuf::new_workspace_rel(SOURCE_PATH).unwrap();
	let original_source = std::fs::read_to_string(source.as_path()).unwrap();
	let _source_guard = SourceRevert {
		path: source.clone(),
		original: original_source,
	};
	let assets_file = AbsPathBuf::new_workspace_rel(ASSETS_FILE).unwrap();
	let original_assets =
		std::fs::read_to_string(assets_file.as_path()).unwrap();
	let _assets_guard = SourceRevert {
		path: assets_file.clone(),
		original: original_assets,
	};

	let mut stack = Stack::new("lightsail-test").with_aws_region("us-west-2");

	// clean up any prior state
	let project = build_project(&stack).unwrap();
	project.force_destroy().await;
	let client = stack.artifacts_client();
	client.bucket().bucket_remove().await.ok();
	assets_bucket(&stack).bucket_remove().await.ok();

	// 1. deploy v1
	info!("step 1: deploying v1");
	deploy(&stack).await.unwrap();

	// 2. verify v1 is live
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 2: verifying v1 at {address}");
	verify_live(&address, MARKER_V1).await.unwrap();

	// 3. verify v1 assets
	info!("step 3: verifying v1 assets");
	verify_assets(&stack, MARKER_V1).await.unwrap();

	// 4-5. modify source and assets to v2, deploy again
	info!("step 4-5: deploying v2");
	swap_version(&source, MARKER_V1, MARKER_V2).unwrap();
	swap_version(&assets_file, MARKER_V1, MARKER_V2).unwrap();
	stack = Stack::new("lightsail-test").with_aws_region("us-west-2");
	deploy(&stack).await.unwrap();

	// 6. verify v2
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 6: verifying v2 at {address}");
	verify_live(&address, MARKER_V2).await.unwrap();

	// 7. verify v2 assets
	info!("step 7: verifying v2 assets");
	verify_assets(&stack, MARKER_V2).await.unwrap();

	// 8. rollback to v1
	info!("step 8: rolling back");
	let client = stack.artifacts_client();
	client.rollback(1).await.unwrap();
	apply_with_current_ledger(&mut stack).await.unwrap();

	// 9. verify v1 after rollback
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 9: verifying v1 after rollback at {address}");
	verify_live(&address, MARKER_V1).await.unwrap();

	// 10. verify v1 assets after rollback
	info!("step 10: verifying v1 assets after rollback");
	verify_assets(&stack, MARKER_V1).await.unwrap();

	// 11. rollforward to v2
	info!("step 11: rolling forward");
	let client = stack.artifacts_client();
	client.rollforward().await.unwrap();
	apply_with_current_ledger(&mut stack).await.unwrap();

	// 12. verify v2 after rollforward
	let project = build_project(&stack).unwrap();
	let address = project.output("public_address").await.unwrap();
	info!("step 12: verifying v2 after rollforward at {address}");
	verify_live(&address, MARKER_V2).await.unwrap();

	// 13. verify v2 assets after rollforward
	info!("step 13: verifying v2 assets after rollforward");
	verify_assets(&stack, MARKER_V2).await.unwrap();

	// 14. destroy
	info!("step 14: destroying");
	let project = build_project(&stack).unwrap();
	project.destroy().await.unwrap();
	let client = stack.artifacts_client();
	client.bucket().bucket_remove().await.ok();
	assets_bucket(&stack).bucket_remove().await.ok();

	// 15. verify dead
	info!("step 15: verifying dead");
	verify_dead(&address).await.unwrap();

	// revert source files (guards also handle this)
	swap_version(&source, MARKER_V2, MARKER_V1).ok();
	swap_version(&assets_file, MARKER_V2, MARKER_V1).ok();
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

fn assets_bucket_block() -> S3BucketBlock {
	S3BucketBlock::new("assets").with_deploy_versioned(true)
}

fn assets_s3_fs_bucket(stack: &Stack) -> S3FsBucket {
	S3FsBucket::new(
		FsBucket::new(WsPathBuf::new(ASSETS_PATH)),
		assets_bucket_block().provider(stack),
	)
}

/// Get the deploy-versioned assets bucket for verification.
fn assets_bucket(stack: &Stack) -> Bucket {
	Bucket::new(assets_bucket_block().provider(stack))
}

/// Build the terraform project for the Lightsail test stack.
fn build_project(stack: &Stack) -> Result<terra::Project> {
	let block = LightsailBlock::default();
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

/// Build, upload artifacts, sync assets, and apply terraform
/// using the deploy action sequence.
async fn deploy(stack: &Stack) -> Result {
	let block = LightsailBlock::default();
	let cargo = CargoBuild::default()
		.with_release(true)
		.with_target(BuildTarget::Zigbuild)
		.with_package("beet_infra")
		.with_example(EXAMPLE_NAME)
		.with_additional_args(vec!["--features".into(), "deploy".into()])
		.into_build_artifact();

	let response = AsyncPlugin::world()
		.spawn((
			stack.clone(),
			assets_s3_fs_bucket(stack),
			assets_bucket_block(),
			exchange_sequence(),
			children![(block, cargo), TofuApplyAction, SyncS3BucketAction,],
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

/// Verify the assets bucket contains the expected version marker.
async fn verify_assets(stack: &Stack, expected: &str) -> Result {
	let bucket = assets_bucket(stack);
	let files = bucket.list().await?;
	info!("assets at deploy {}: {:?}", stack.deploy_id(), files);
	files
		.iter()
		.any(|path| path.to_string_lossy().contains("index.html"))
		.xpect_true();
	let bytes = bucket.get(&RelPath::new("index.html")).await?;
	let content = String::from_utf8(bytes.to_vec())?;
	content.contains(expected).xpect_true();
	info!(
		"verified assets contain '{expected}' at deploy {}",
		stack.deploy_id()
	);
	Ok(())
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

/// Modify a file to use a different version marker.
fn swap_version(path: &AbsPathBuf, from: &str, to: &str) -> Result {
	let content = std::fs::read_to_string(path.as_path())?;
	let updated = content.replacen(from, to, 1);
	if content == updated {
		bevybail!("marker '{from}' not found in {}", path.display());
	}
	std::fs::write(path.as_path(), &updated)?;
	Ok(())
}
