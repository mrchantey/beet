//! Shared utilities for Lambda, Lightsail, and Fargate integration tests.
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;

pub const ASSETS_PATH: &str = "examples/infra/assets";

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

/// Owns isolated temp assets so tests can run in parallel without
/// interfering via the shared `examples/infra/assets/` directory.
/// Fields drop in declaration order: source/assets reverts first,
/// then the temp dir is removed.
pub struct IsolatedTestGuards {
	/// Path to the real source file (eg `examples/fargate_test.rs`).
	pub source: AbsPathBuf,
	/// Path to `index.html` inside the isolated temp assets dir.
	pub assets_file: AbsPathBuf,
	/// Isolated temp assets directory used by `assets_s3_fs_bucket`.
	pub assets_dir: AbsPathBuf,
	// reverts first
	_source_guard: SourceRevert,
	// reverts second (temp file still alive)
	_assets_guard: SourceRevert,
	// deleted last
	_temp_dir: TempDir,
}

/// Create isolated test guards: reverts real source on drop and
/// copies assets into a per-test temp dir to prevent concurrent modifications.
pub fn setup_isolated_test_guards(
	source_path: &str,
) -> Result<IsolatedTestGuards> {
	// real source file guard
	let source = AbsPathBuf::new_workspace_rel(source_path)?;
	let original_source = std::fs::read_to_string(source.as_path())?;
	let source_guard = SourceRevert {
		path: source.clone(),
		original: original_source,
	};

	// isolated temp assets dir
	let temp_dir = TempDir::new_workspace()?;
	let src_assets = AbsPathBuf::new_workspace_rel(ASSETS_PATH)?;
	for entry in std::fs::read_dir(src_assets.as_path())? {
		let entry = entry?;
		if entry.file_type()?.is_file() {
			std::fs::copy(
				entry.path(),
				temp_dir.path().as_path().join(entry.file_name()),
			)?;
		}
	}
	let assets_dir = temp_dir.path().clone();
	let assets_file = AbsPathBuf::new(assets_dir.join("index.html"))?;
	let original_assets = std::fs::read_to_string(&assets_file.as_path())?;
	let assets_guard = SourceRevert {
		path: assets_file.clone(),
		original: original_assets,
	};

	Ok(IsolatedTestGuards {
		source,
		assets_file,
		assets_dir,
		_source_guard: source_guard,
		_assets_guard: assets_guard,
		_temp_dir: temp_dir,
	})
}

/// Create the assets bucket block used across all tests.
pub fn assets_bucket_block() -> S3BucketBlock {
	S3BucketBlock::new("assets").with_deploy_versioned(true)
}

/// Create the S3FsBucket for syncing local assets to S3.
/// `assets_dir` is typically the isolated temp dir from [`IsolatedTestGuards`].
pub fn assets_s3_fs_bucket(
	stack: &Stack,
	assets_dir: &AbsPathBuf,
) -> S3FsBucket {
	S3FsBucket::new(
		FsBucket::new(assets_dir.clone()),
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
