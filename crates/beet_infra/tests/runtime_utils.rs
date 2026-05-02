//! Shared utilities for Lambda, Lightsail, and Fargate integration tests.
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;

pub const ASSETS_PATH: &str = "examples/infra/assets";
pub const ASSETS_FILE: &str = "examples/infra/assets/index.html";
pub const MARKER_V1: &str = "test-v1";
pub const MARKER_V2: &str = "test-v2";

/// Guard that reverts source file changes on drop.
pub struct SourceRevert {
	pub path: AbsPathBuf,
	pub original: String,
}

impl Drop for SourceRevert {
	fn drop(&mut self) {
		std::fs::write(self.path.as_path(), &self.original).ok();
	}
}

/// Create the assets bucket block used across all tests.
pub fn assets_bucket_block() -> S3BucketBlock {
	S3BucketBlock::new("assets").with_deploy_versioned(true)
}

/// Create the S3FsBucket for syncing local assets to S3.
pub fn assets_s3_fs_bucket(stack: &Stack) -> S3FsBucket {
	S3FsBucket::new(
		FsBucket::new(WsPathBuf::new(ASSETS_PATH)),
		assets_bucket_block().provider(stack),
	)
}

/// Get the deploy-versioned assets bucket for verification.
pub fn assets_bucket(stack: &Stack) -> Bucket {
	Bucket::new(assets_bucket_block().provider(stack))
}

/// Re-apply terraform with the current ledger deploy_id.
/// Used after rollback/rollforward to update the deployed resource.
pub async fn apply_with_current_ledger<F>(
	stack: &mut Stack,
	build_project: F,
) -> Result<String>
where
	F: FnOnce(&Stack) -> Result<terra::Project>,
{
	let client = stack.artifacts_client();
	let ledger = client
		.current_ledger()
		.await?
		.ok_or_else(|| bevyhow!("no current ledger"))?;
	stack.update_from_ledger(&ledger);
	let project = build_project(stack)?;
	project.apply().await
}

/// Verify the assets bucket contains the expected version marker.
pub async fn verify_assets(stack: &Stack, expected: &str) -> Result {
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

/// Modify a file to use a different version marker.
pub fn swap_version(path: &AbsPathBuf, from: &str, to: &str) -> Result {
	let content = std::fs::read_to_string(path.as_path())?;
	let updated = content.replacen(from, to, 1);
	if content == updated {
		bevybail!("marker '{from}' not found in {}", path.display());
	}
	std::fs::write(path.as_path(), &updated)?;
	Ok(())
}

/// Verify the deployed endpoint returns the expected version marker.
/// Retries with exponential backoff.
pub async fn verify_live(
	url: &str,
	expected: &str,
	max_attempts: u32,
	sleep_secs: u64,
) -> Result {
	let mut last_err = bevyhow!("no attempts made");
	for attempt in 0..max_attempts {
		match Request::get(url).send().await {
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
		time_ext::sleep(Duration::from_secs(sleep_secs)).await;
	}
	Err(last_err)
}

/// Verify the endpoint is no longer reachable after destroy.
pub async fn verify_dead(
	url: &str,
	initial_wait_secs: u64,
	max_attempts: u32,
	sleep_secs: u64,
) -> Result {
	time_ext::sleep(Duration::from_secs(initial_wait_secs)).await;
	for _ in 0..max_attempts {
		match Request::get(url).send().await {
			Err(_) => return Ok(()),
			Ok(res) if !res.status().is_success() => return Ok(()),
			Ok(_) => {}
		}
		time_ext::sleep(Duration::from_secs(sleep_secs)).await;
	}
	bevybail!("endpoint still reachable after destroy")
}

/// Common cleanup: destroy project and remove artifacts bucket.
/// The artifacts bucket is not managed by terraform, so we clean it manually.
/// The assets bucket IS managed by terraform (S3BucketBlock), so terraform cleans it.
pub async fn cleanup(stack: &Stack, project: terra::Project) -> Result {
	project.destroy().await?;
	// only clean artifacts bucket - not managed by terraform
	let client = stack.artifacts_client();
	client.bucket().bucket_remove().await.ok();
	Ok(())
}

/// Setup source guards for test files.
pub fn setup_source_guards(
	source_path: &str,
	assets_file: &str,
) -> Result<(AbsPathBuf, SourceRevert, AbsPathBuf, SourceRevert)> {
	let source = AbsPathBuf::new_workspace_rel(source_path)?;
	let original_source = std::fs::read_to_string(source.as_path())?;
	let source_guard = SourceRevert {
		path: source.clone(),
		original: original_source,
	};

	let assets = AbsPathBuf::new_workspace_rel(assets_file)?;
	let original_assets = std::fs::read_to_string(assets.as_path())?;
	let assets_guard = SourceRevert {
		path: assets.clone(),
		original: original_assets,
	};

	Ok((source, source_guard, assets, assets_guard))
}

/// Clean up any prior state before test starts.
/// Terraform should handle all infrastructure cleanup, we only need to clean
/// the artifacts bucket which is not managed by terraform.
pub async fn cleanup_prior_state(stack: &Stack, project: terra::Project) {
	info!("cleanup_prior_state: calling force_destroy");
	project.force_destroy().await;
	info!("cleanup_prior_state: removing artifacts bucket");
	stack.artifacts_client().bucket().bucket_remove().await.ok();
	info!("cleanup_prior_state: complete");
}
