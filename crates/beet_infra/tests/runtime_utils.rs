//! Shared utilities for Lambda, Lightsail, and Fargate integration tests.
// each test binary that includes this module uses a different subset of helpers.
#![allow(dead_code)]

beet_core::test_main!();

use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;

pub const ASSETS_PATH: &str = "examples/infra/assets";

pub const MARKER_V1: &str = "test-v1";
pub const MARKER_V2: &str = "test-v2";

/// Initialize the test logger, ignoring the error if already set.
/// Multiple ignored tests can share a process (eg `--ignored`), so the
/// global logger may already be initialized by a prior test.
pub fn init_logger() { pretty_env_logger::try_init().ok(); }

/// Guard that reverts source file changes on drop.
pub struct SourceRevert {
	pub path: AbsPath,
	pub original: String,
}

impl Drop for SourceRevert {
	fn drop(&mut self) { fs_ext::write(&self.path, &self.original).ok(); }
}

/// Owns isolated temp assets so tests can run in parallel without
/// interfering via the shared `examples/infra/assets/` directory.
/// Fields drop in declaration order: source/assets reverts first,
/// then the temp dir is removed.
pub struct IsolatedTestGuards {
	/// Path to the real source file (eg `examples/fargate_test.rs`).
	pub source: AbsPath,
	/// Path to `index.html` inside the isolated temp assets dir.
	pub assets_file: AbsPath,
	/// Isolated temp assets directory used by `assets_s3_fs_store`.
	pub assets_dir: AbsPath,
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
	let source = AbsPath::new_workspace_rel(source_path)?;
	let original_source = fs_ext::read_to_string(&source)?;
	let source_guard = SourceRevert {
		path: source.clone(),
		original: original_source,
	};

	// isolated temp assets dir
	let temp_dir = TempDir::new_workspace()?;
	let src_assets = AbsPath::new_workspace_rel(ASSETS_PATH)?;
	for entry in ReadDir::files(&src_assets)? {
		fs_ext::copy(
			&entry,
			temp_dir
				.path()
				.join(path_ext::file_name(&entry)?.to_string_lossy()),
		)?;
	}
	let assets_dir = temp_dir.path().clone();
	let assets_file = assets_dir.join("index.html");
	let original_assets = fs_ext::read_to_string(&assets_file)?;
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

/// The identity + launch pair every lifecycle step threads through, so one run
/// publishes every artifact under one deploy id and a re-deploy is an explicit
/// new [`Deployment`] rather than a rebuilt stack.
pub struct TestDeploy {
	/// The declaration, which is what a world spawns so descendants resolve it
	/// by ancestry.
	pub stack: Stack,
	pub deployment: Deployment,
}

impl TestDeploy {
	/// A first deploy of `app_name`, pinned to the region the live tests use.
	pub fn new(app_name: &str) -> Self {
		Self {
			stack: Stack::new(app_name).with_region("us-west-2"),
			deployment: Deployment::default(),
		}
	}

	/// The next deploy of the same app: same identity, new launch.
	pub fn redeploy(&self) -> Self {
		Self {
			stack: self.stack.clone(),
			deployment: Deployment::default(),
		}
	}

	/// The composed identity, ie what a name resolves against outside a world.
	pub fn resolved(&self) -> ResolvedStack {
		self.stack.resolve(&PackageConfig::default())
	}

	/// The client of the stack's repo store, exactly as
	/// `RepoStoreQuery::artifacts_client` builds it in a deploy.
	pub fn artifacts_client(&self) -> ArtifactsClient {
		ArtifactsClient::new(
			BlobStore::from_uri(repo_store(self).root()).unwrap(),
			ArtifactLedger::new(
				*self.deployment.deploy_id(),
				self.deployment.deploy_timestamp().to_string(),
			),
		)
	}
}

/// The repo store declaration every test stack carries: the bucket the site
/// is published into and the ledger lives in, deploy-versioned by default.
pub fn repo_store_block() -> impl Bundle {
	(S3BucketBlock::new("repo"), RepoStoreBlock)
}

/// The erased half of [`repo_store_block`] under `deploy`'s stack.
fn repo_store(deploy: &TestDeploy) -> ErasedStoreBlock {
	ErasedStoreBlock::new(&S3BucketBlock::new("repo"), &deploy.resolved())
}

/// The publish steps of a deploy: stage `site_dir` and mirror it into the
/// repo store, so the test's isolated copy of the site is what gets served.
pub fn publish_site(site_dir: &AbsPath) -> Result<impl Bundle> {
	(
		RepoStage::new(site_dir.into_ws_path()?),
		RepoSync::default(),
	)
		.xok()
}

/// Build the terraform project for `block` deployed beside the repo store,
/// rendered through the same [`DeployRender`] schedule the deploy runs.
pub fn render_test_project(
	deploy: &TestDeploy,
	block: impl Bundle,
) -> Result<terra::Project> {
	let mut world = InfraPlugin.into_world();
	world.insert_resource(deploy.deployment.clone());
	world.init_resource::<PackageConfig>();
	let root = world
		.spawn(deploy.stack.clone())
		.with_children(|parent| {
			parent.spawn(block);
			parent.spawn(repo_store_block());
		})
		.id();
	RenderScope::render(&mut world, root)?.project()
}

/// The document root of `deploy`'s version in the repo store, exactly as the
/// declaration projects it.
pub fn repo_uri(deploy: &TestDeploy) -> StoreUri {
	repo_store(deploy).store_uri(Some(deploy.deployment.deploy_id()))
}

/// The published document of `deploy`'s version, for verification.
pub fn repo_document(deploy: &TestDeploy) -> BlobStore {
	BlobStore::from_uri(&repo_uri(deploy)).unwrap()
}

/// Re-apply terraform with the current ledger deploy_id.
/// Used after rollback/rollforward to update the deployed resource.
pub async fn apply_with_current_ledger<F>(
	deploy: &mut TestDeploy,
	build_project: F,
) -> Result<String>
where
	F: FnOnce(&TestDeploy) -> Result<terra::Project>,
{
	let ledger = deploy
		.artifacts_client()
		.current_ledger()
		.await?
		.ok_or_else(|| bevyhow!("no current ledger"))?;
	deploy.deployment.update_from_ledger(&ledger);
	build_project(deploy)?.apply().await
}

/// Verify the published document of `deploy`'s version carries the expected
/// version marker.
pub async fn verify_assets(deploy: &TestDeploy, expected: &str) -> Result {
	let store = repo_document(deploy);
	let files = store.list().await?;
	let deploy_id = deploy.deployment.deploy_id();
	info!("document at deploy {deploy_id}: {files:?}");
	files
		.iter()
		.any(|path| path.contains("index.html"))
		.xpect_true();
	let bytes = store.get(&RelPath::new("index.html")).await?;
	let content = String::from_utf8(bytes.to_vec())?;
	content.contains(expected).xpect_true();
	info!("verified assets contain '{expected}' at deploy {deploy_id}");
	Ok(())
}

/// Modify a file to use a different version marker.
pub fn swap_version(path: &AbsPath, from: &str, to: &str) -> Result {
	let content = fs_ext::read_to_string(path)?;
	let updated = content.replacen(from, to, 1);
	if content == updated {
		bevybail!("marker '{from}' not found in {path}");
	}
	fs_ext::write(path, &updated)?;
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

/// Clean up any prior state before test starts. Terraform owns every
/// resource including the repo store, so this is the forced destroy plus a
/// best-effort sweep of a repo bucket an interrupted run left behind.
pub async fn cleanup_prior_state(deploy: &TestDeploy, project: terra::Project) {
	info!("cleanup_prior_state: calling tofu_destroy --force");
	project.tofu_destroy(true).await.ok();
	info!("cleanup_prior_state: removing the repo store");
	deploy.artifacts_client().store().store_remove().await.ok();
	info!("cleanup_prior_state: complete");
}
