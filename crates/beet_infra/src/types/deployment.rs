//! The one-launch mechanics of a deploy: where its state lives, where it works,
//! and the id every artifact it publishes is keyed by.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The work-dir guard [`Deployment::default_local`] returns: a real [`TempDir`]
/// on native, nothing on wasm where the config-only tests never write to it.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) type TestWorkDir = TempDir;
#[cfg(all(test, target_arch = "wasm32"))]
pub(crate) struct TestWorkDir;

/// Everything about THIS deploy run, as [`BootstrapConfig`] is everything about
/// this process launch. Deliberately not `Reflect`: none of it is authored, so
/// none of it belongs in markup. Identity is the other half and lives on
/// [`Stack`].
///
/// One per world, beside however many stacks a document declares: a stage stack
/// and a shared stack share this launch's id and work directory, and still
/// compose distinct state paths, since every path composes through the stack's
/// own identity.
#[derive(Debug, Clone, Resource, Get, Set, SetWith)]
pub struct Deployment {
	/// Unique deploy identifier, regenerated for each deployment unless the
	/// launch names one (a beet process spawned by a deploy inherits it).
	deploy_id: Uuid,
	/// Timestamp for this deployment, in the same inherit-else-generate shape as
	/// [`Self::deploy_id`].
	deploy_timestamp: String,
	/// Where the tofu state for every stack in this deploy is kept.
	#[set_with(into)]
	backend: StackBackend,
	/// The opentofu working directory, `target/infra/<app>` when unset (see
	/// [`Self::work_directory`]). A test points it at a temp dir.
	#[get(skip)]
	#[set_with(unwrap_option)]
	work_directory: Option<WsPathBuf>,
	/// A suffix appended to the state backend key, making the final key
	/// `app-name--stage--tofu.tfstate`.
	state_suffix: SmolStr,
	/// A suffix appended to the artifact bucket name, making the final bucket
	/// `app-name--stage--artifacts`.
	artifact_bucket_suffix: SmolStr,
}

/// The deploy identity flows from the process [`BootstrapConfig`] (`--deploy-id`
/// and friends), so it reaches every deploy verb without per-template threading,
/// and a beet process spawned by a deploy publishes under the same id.
impl Default for Deployment {
	fn default() -> Self {
		let config = BootstrapConfig::get();
		Self {
			deploy_id: config
				.deploy_id
				.as_deref()
				.and_then(|id| Uuid::parse_str(id).ok())
				.unwrap_or_else(Uuid::now_v7),
			deploy_timestamp: config
				.deploy_timestamp
				.as_ref()
				.map(|stamp| stamp.to_string())
				.unwrap_or_else(crate::types::artifacts::now_timestamp),
			backend: default(),
			work_directory: None,
			state_suffix: "tofu.tfstate".into(),
			artifact_bucket_suffix: "artifacts".into(),
		}
	}
}

impl Deployment {
	/// Point this deploy at the version `ledger` names, so a rollback re-applies
	/// the artifacts of an earlier run rather than of this one.
	pub fn update_from_ledger(&mut self, ledger: &ArtifactLedger) {
		self.deploy_id = ledger.deploy_id;
		self.deploy_timestamp = ledger.timestamp.clone();
	}

	/// The opentofu working directory for `stack`, ie `target/infra/beet-site`.
	/// Per app rather than per stage, since one app's stacks share a checkout.
	pub fn work_directory(&self, stack: &ResolvedStack) -> WsPathBuf {
		self.work_directory.clone().unwrap_or_else(|| {
			WsPathBuf::new(format!("target/infra/{}", stack.app_name()))
		})
	}

	/// The state backend path for `stack`, ie `my-app--prod--tofu.tfstate`. It
	/// composes through the stack's identity, so two stacks sharing this launch
	/// still write distinct state.
	pub fn backend_path(&self, stack: &ResolvedStack) -> SmolPath {
		SmolPath::new(stack.resource_name(self.state_suffix.clone()))
	}

	/// The S3 bucket name for `stack`'s artifacts storage.
	pub fn artifact_bucket_name(&self, stack: &ResolvedStack) -> String {
		stack.resource_name(self.artifact_bucket_suffix.clone())
	}

	/// The S3 key for an artifact in this deployment.
	pub fn artifact_key(&self, label: &str) -> String {
		format!("versions/{}/{label}", self.deploy_id)
	}

	/// Create an artifacts client for `stack`'s artifact bucket, in the same
	/// provider family as the state backend: local state stores artifacts in a
	/// sibling directory, S3 state in an S3 bucket in the stack's region.
	pub fn artifacts_client(&self, stack: &ResolvedStack) -> ArtifactsClient {
		let provider = self
			.backend
			.bucket_provider(&self.artifact_bucket_name(stack), stack.region());
		ArtifactsClient::new(
			BlobStore::new(provider),
			ArtifactLedger::new(self.deploy_id, self.deploy_timestamp.clone()),
		)
	}

	/// Initialize `stack`'s config with the corresponding backend.
	pub fn create_config(&self, stack: &ResolvedStack) -> terra::Config {
		let key = self.backend_path(stack).to_string();
		terra::Config::default().with_backend(self.backend.to_json(&key))
	}

	/// The blob holding `stack`'s tofu state.
	pub fn state_file(&self, stack: &ResolvedStack) -> Blob {
		self.backend
			.provider()
			.erased_blob(self.backend_path(stack))
	}

	/// A deploy with a local backend and a temporary work directory for testing.
	/// The directory is removed on drop. On wasm it is a fixed pseudo path: the
	/// config-only tests never touch the fs, and the two `Project::validate`
	/// tests that do are native-gated.
	#[cfg(test)]
	pub fn default_local() -> (Self, TestWorkDir) {
		#[cfg(not(target_arch = "wasm32"))]
		let (dir, path) = {
			let dir = TempDir::new_ws().unwrap();
			let path = dir.path().into_ws_path().unwrap();
			(dir, path)
		};
		#[cfg(target_arch = "wasm32")]
		let (dir, path) = (TestWorkDir, WsPathBuf::new("target/infra/test"));

		(
			Self {
				backend: LocalBackend::default().into(),
				work_directory: Some(path),
				..default()
			},
			dir,
		)
	}
}
