use crate::prelude::*;
use crate::terra::Project;
use beet_core::prelude::*;
use beet_net::prelude::*;

#[derive(Debug, Clone, Get, SetWith, Component)]
pub struct Stack {
	/// The app name, defaults to `CARGO_PKG_NAME`
	app_name: SmolStr,
	/// The deployment stage, defaults to `dev`
	stage: SmolStr,
	/// Name of the production stage, which often receives
	/// special treatment like bucket locking and no subdomain.
	prod_stage: SmolStr,
	/// Allow reconfiguring the backend without migrating state
	reconfigure: bool,
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
}

impl Default for Stack {
	fn default() -> Self {
		let app_name = std::env::var("CARGO_PKG_NAME").unwrap();
		Self::new(app_name)
	}
}
impl Stack {
	pub fn new(app_name: impl Into<SmolStr>) -> Self {
		let app_name = app_name.into();
		let work_directory = WsPathBuf::new(format!("target/infra/{app_name}"));
		Self {
			app_name,
			work_directory,
			state_suffix: "tofu.tfstate".into(),
			stage: "dev".into(),
			prod_stage: "prod".into(),
			params: default(),
			artifact_bucket_suffix: "artifacts".into(),
			reconfigure: false,
			backend: default(),
			aws_region: crate::bindings::aws::region::DEFAULT.into(),
		}
	}
	/// Create a stack with a local backend and a temporary directory for testing.
	/// The directory will be removed on drop.
	#[cfg(test)]
	pub fn default_local() -> (Self, TempDir) {
		let dir = TempDir::new_ws().unwrap();
		let path = dir.path().into_ws_path().unwrap();

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
	pub fn backend_path(&self) -> RelPath {
		RelPath::new(
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

	/// Create an artifacts client for this stack's artifact bucket.
	pub fn artifacts_client(&self) -> ArtifactsClient {
		cfg_if! {
			if #[cfg(feature = "aws")] {
				let provider = beet_net::prelude::S3Bucket::new(
					self.artifact_bucket_name(),
					self.aws_region().clone(),
				);
				ArtifactsClient::new(Bucket::new(provider))
			} else {
				panic!("the `aws` feature is required for artifact operations")
			}
		}
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
	#[cfg(feature = "bindings_aws_common")]
	s3_buckets: Query<'w, 's, &'static S3BucketBlock>,
}

impl<'w, 's> StackQuery<'w, 's> {
	/// Finds the stack in ancestors and
	/// builds a config of all block descendents.
	/// Sets the AWS provider region from the nearest [`AwsStack`] ancestor,
	/// ensuring the tofu config and Rust SDK use the same region.
	pub fn build_project(&self, entity: Entity) -> Result<terra::Project> {
		let (root, stack) = self.stacks.get(entity)?;
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
		Ok(Project::new(&stack, config))
	}

	/// Create an artifacts client for the stack at the given entity.
	pub fn artifacts_client(&self, entity: Entity) -> Result<ArtifactsClient> {
		let (_, stack) = self.stacks.get(entity)?;
		stack.artifacts_client().xok()
	}

	/// Get the provider from an [`S3Bucket`] on this entity
	#[cfg(all(feature = "aws", feature = "bindings_aws_common"))]
	pub fn s3_provider(&self, entity: Entity) -> Result<S3Bucket> {
		let (_, stack) = self.stacks.get(entity)?;
		let bucket = self.s3_buckets.get(entity)?;
		bucket.provider(stack).xok()
	}
}
