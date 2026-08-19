use crate::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::terra::Project;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The work-dir guard [`Stack::default_local`] returns: a real [`TempDir`] on
/// native, nothing on wasm where the config-only tests never write to it.
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(crate) type TestWorkDir = TempDir;
#[cfg(all(test, target_arch = "wasm32"))]
pub(crate) struct TestWorkDir;

#[derive(Debug, Clone, Get, SetWith, Component)]
pub struct Stack {
	/// The app name, defaults to `CARGO_PKG_NAME`
	app_name: SmolStr,
	/// The deployment stage, namespacing every resource. Resolved by [`Stack::new`]
	/// from the `--stage=<x>` cli arg, else the `BEET_STAGE` env var, else `dev`.
	stage: SmolStr,
	/// Name of the production stage, which often receives
	/// special treatment like bucket locking and no subdomain.
	prod_stage: SmolStr,
	/// A suffix to append to the state backend, defaults to `tofu.tfstate`,
	/// making the final state key `app-name--stage--state-suffix`
	state_suffix: SmolStr,
	/// A suffix to append to the artifact bucket name, defaults to `artifacts`,
	/// making the final bucket name `app-name--stage--artifacts`
	artifact_bucket_suffix: SmolStr,
	/// The opentofu directory for creating
	/// and deploying infrastructure config.
	work_directory: WsPathBuf,
	#[set_with(into)]
	backend: StackBackend,
	/// The default aws region to use
	aws_region: SmolStr,
	/// Additional parameters, some of which
	/// may be required by a config generator
	params: MultiMap<SmolStr, SmolStr>,
	/// Unique deploy identifier, regenerated for each deployment.
	deploy_id: Uuid,
	/// Timestamp for this deployment.
	deploy_timestamp: String,
}

impl Default for Stack {
	fn default() -> Self {
		let app_name = env_ext::var("CARGO_PKG_NAME").unwrap();
		Self::new(app_name)
	}
}
impl Stack {
	pub fn new(app_name: impl Into<SmolStr>) -> Self {
		let app_name = app_name.into();
		let work_directory = WsPathBuf::new(format!("target/infra/{app_name}"));

		// the deploy identity and stage flow from the process `BootstrapConfig`
		// (`--stage` / `BEET_STAGE` and friends), so they reach every stack in
		// every scene without per-template threading.
		let config = BootstrapConfig::get();

		let deploy_id = config
			.deploy_id
			.as_deref()
			.and_then(|id| Uuid::parse_str(id).ok())
			.unwrap_or_else(Uuid::now_v7);

		let deploy_timestamp = config
			.deploy_timestamp
			.as_ref()
			.map(|stamp| stamp.to_string())
			.unwrap_or_else(crate::types::artifacts::now_timestamp);

		let stage = config.stage.clone();

		Self {
			app_name,
			work_directory,
			state_suffix: "tofu.tfstate".into(),
			stage,
			prod_stage: "prod".into(),
			params: default(),
			artifact_bucket_suffix: "artifacts".into(),
			backend: default(),
			aws_region: crate::bindings::aws::region::DEFAULT.into(),
			deploy_id,
			deploy_timestamp,
		}
	}
	pub fn update_from_ledger(&mut self, ledger: &ArtifactLedger) {
		self.deploy_id = ledger.deploy_id;
		self.deploy_timestamp = ledger.timestamp.clone();
	}

	/// Create a stack with a local backend and a temporary directory for testing.
	/// The directory will be removed on drop. On wasm the directory is a fixed
	/// pseudo path: the config-only tests never touch the fs, and the two
	/// `Project::validate` tests that do are native-gated.
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
				work_directory: path,
				..default()
			},
			dir,
		)
	}

	pub fn is_production(&self) -> bool { self.stage == self.prod_stage }

	// pub fn bucket_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
	// 	self.resource_ident("buckets", label)
	// }
	// pub fn iam_role_slug(&self, label: impl Into<SmolStr>) -> terra::Ident {
	// 	self.resource_ident("iam-roles", label)
	// }

	pub fn resource_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
		terra::Ident::new(self.app_name.clone(), self.stage.clone(), label)
	}

	/// The state backend path, ie `my-app--prod--tofu.tfstate`.
	pub fn backend_path(&self) -> SmolPath {
		SmolPath::new(
			self.resource_ident(self.state_suffix.clone())
				.primary_identifier()
				.to_string(),
		)
	}

	/// The S3 bucket name for artifacts storage.
	pub fn artifact_bucket_name(&self) -> String {
		self.resource_ident(self.artifact_bucket_suffix.clone())
			.primary_identifier()
			.to_string()
	}

	/// The S3 key for an artifact in this deployment.
	pub fn artifact_key(&self, label: &str) -> String {
		format!("versions/{}/{label}", self.deploy_id)
	}

	/// Create an artifacts client for this stack's artifact bucket, in the same
	/// provider family as the state backend: local state stores artifacts in a
	/// sibling directory, S3 state in an S3 bucket in [`Self::aws_region`].
	pub fn artifacts_client(&self) -> ArtifactsClient {
		let provider = self
			.backend
			.bucket_provider(&self.artifact_bucket_name(), &self.aws_region);
		ArtifactsClient::new(
			BlobStore::new(provider),
			ArtifactLedger::new(self.deploy_id, self.deploy_timestamp.clone()),
		)
	}

	/// Initialize a config with the corresponding backend.
	pub fn create_config(&self) -> terra::Config {
		let key = self.backend_path().to_string();
		terra::Config::default().with_backend(self.backend().to_json(&key))
	}

	pub fn state_file(&self) -> Blob {
		self.backend.provider().erased_blob(self.backend_path())
	}
}

#[derive(SystemParam)]
pub struct StackQuery<'w, 's> {
	stacks: AncestorQuery<'w, 's, (Entity, &'static Stack)>,
	blocks: Query<'w, 's, (EntityRef<'static>, &'static ErasedBlock)>,
	children: Query<'w, 's, &'static Children>,
	stores: Query<'w, 's, &'static BlobStore>,
}

impl<'w, 's> StackQuery<'w, 's> {
	/// The nearest ancestor carrying a [`Stack`], and the stack itself: the
	/// root every block, artifact and verb under one deploy resolves against.
	pub fn root(&self, entity: Entity) -> Result<(Entity, &Stack)> {
		self.stacks.get(entity)
	}

	/// Every descendant of `entity`'s stack root, inclusive. The one traversal
	/// a deploy step uses to find what was declared alongside it.
	pub fn declared(
		&self,
		entity: Entity,
	) -> Result<impl Iterator<Item = Entity> + use<'_>> {
		let (root, _) = self.root(entity)?;
		self.children.iter_descendants_inclusive(root).xok()
	}

	/// Finds the stack in ancestors and
	/// builds a config of all block descendents.
	/// Sets the AWS provider region from the nearest [`AwsStack`] ancestor,
	/// ensuring the tofu config and Rust SDK use the same region.
	///
	/// This is the whole definition step, and it is target-agnostic: a wasm
	/// consumer authors blocks and builds the config here, then serializes it for
	/// a host that can apply it (see [`build_project`](Self::build_project),
	/// which is the same config wrapped in the native tofu driver).
	pub fn build_config(&self, entity: Entity) -> Result<(&Stack, terra::Config)> {
		let (root, stack) = self.root(entity)?;
		let mut config = stack.create_config();
		let region = stack.aws_region();
		config.add_provider_config(
			&terra::Provider::AWS,
			&serde_json::json!({ "region": region }),
		)?;
		for (child, block) in self
			.children
			.iter_descendants_inclusive(root)
			.filter_map(|child| self.blocks.get(child).ok())
		{
			block.apply_to_config(&child, stack, &mut config)?;
		}
		Ok((stack, config))
	}

	/// [`build_config`](Self::build_config) wrapped in the tofu driver that
	/// applies it, hence native-only.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn build_project(&self, entity: Entity) -> Result<terra::Project> {
		let (stack, config) = self.build_config(entity)?;
		Ok(Project::new(stack, config))
	}

	/// Create an artifacts client for the stack at the given entity.
	pub fn artifacts_client(&self, entity: Entity) -> Result<ArtifactsClient> {
		let (_, stack) = self.root(entity)?;
		stack.artifacts_client().xok()
	}

	/// Collect artifact entries from block descendants.
	/// Returns `(BuildArtifact, artifact_label)` for each block
	/// that has both a [`BuildArtifact`] and an artifact label.
	#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
	pub fn collect_artifacts(
		&self,
		entity: Entity,
	) -> Result<Vec<(BuildArtifact, SmolStr)>> {
		let mut pairs = Vec::new();
		for child in self.declared(entity)? {
			if let Ok((entity_ref, block)) = self.blocks.get(child) {
				if let Some(label) = block.artifact_label() {
					if let Some(artifact) = entity_ref.get::<BuildArtifact>() {
						pairs.push((artifact.clone(), SmolStr::from(label)));
					}
				}
			}
		}
		Ok(pairs)
	}

	/// Collect all [`Variable`] declarations from block descendants.
	#[cfg(feature = "deploy")]
	pub fn collect_variables(&self, entity: Entity) -> Result<Vec<Variable>> {
		let mut variables = Vec::new();
		for child in self.declared(entity)? {
			if let Ok((_, block)) = self.blocks.get(child) {
				variables.extend_from_slice(block.variables());
			}
		}
		Ok(variables)
	}

	/// Get the [`BlobStore`] component from this entity.
	pub fn store(&self, entity: Entity) -> Result<&BlobStore> {
		self.stores.get(entity)?.xok()
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn stage_defaults_to_dev() {
		// the beet test runner passes no `--stage`/`BEET_STAGE`, so a fresh stack
		// resolves the `dev` default and is not production.
		let stack = Stack::new("x");
		stack.stage().xpect_eq("dev");
		stack.is_production().xpect_false();
	}

	#[beet_core::test]
	fn prod_stage_is_production() {
		// the `prod` stage (what `--stage=prod` resolves to) marks production,
		// flipping the stage-aware paths (eg the beet-site apex dns).
		Stack::new("x")
			.with_stage("prod")
			.is_production()
			.xpect_true();
	}
}
