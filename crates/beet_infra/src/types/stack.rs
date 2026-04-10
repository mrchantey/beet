use crate::prelude::*;
use crate::terra::Project;
use beet_core::prelude::*;
#[cfg(feature = "aws")]
use beet_net::prelude::*;

#[derive(Debug, Clone, Get, SetWith, Component)]
pub struct Stack {
	/// The app name, defaults to `CARGO_PKG_NAME`
	app_name: SmolStr,
	/// The deployment stage, defaults to `dev`
	stage: SmolStr,
	/// Additional parameters, some of which
	/// may be required by a config generator
	params: MultiMap<SmolStr, SmolStr>,
	/// Name of the production stage, which often receives
	/// special treatment like bucket locking and no subdomain.
	prod_stage: SmolStr,
	/// Allow reconfiguring the backend without migrating state
	reconfigure: bool,
	/// A suffix to append to the state backend, defaults to `tofu.tfstate`,
	/// making the final state key `app-name--stage--state-suffix`
	state_suffix: SmolStr,
	/// The opentofu directory for creating
	/// and deploying infrastructure config.
	work_directory: WsPathBuf,
	#[set_with(into)]
	backend: StackBackend,
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
			reconfigure: false,
			backend: S3Backend::default().into(),
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
				.clone(),
		)
	}

	/// Initialize a config with the corresponding backend.
	pub fn create_config(&self) -> terra::Config {
		let key = self.backend_path().to_string();
		terra::Config::default().with_backend(self.backend().to_json(&key))
	}

	#[cfg(feature = "aws")]
	pub fn state_file(&self) -> Blob {
		self.backend.provider().blob(self.backend_path())
	}
}


#[derive(SystemParam)]
pub struct StackQuery<'w, 's> {
	stacks: AncestorQuery<
		'w,
		's,
		(Entity, &'static Stack, Option<&'static AwsStack>),
	>,
	blocks: Query<'w, 's, (EntityRef<'static>, &'static ErasedBlock)>,
	s3_buckets: Query<'w, 's, &'static S3BucketBlock>,
	children: Query<'w, 's, &'static Children>,
}

impl<'w, 's> StackQuery<'w, 's> {
	/// Finds the stack in ancestors and
	/// builds a config of all block descendents.
	/// Sets the AWS provider region from the nearest [`AwsStack`] ancestor,
	/// ensuring the tofu config and Rust SDK use the same region.
	pub fn build_project(&self, entity: Entity) -> Result<terra::Project> {
		let (root, stack, aws_stack) = self.stacks.get(entity)?;
		let mut config = stack.create_config();
		let region =
			aws_stack.map_or(AwsStack::DEFAULT_REGION, |s| s.default_region());
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

	#[cfg(feature = "aws")]
	pub fn s3_provider(&self, entity: Entity) -> Result<S3Provider> {
		let (_, stack, aws_stack) = self.stacks.get(entity)?;
		let bucket = self.s3_buckets.get(entity)?;
		bucket.provider(stack, aws_stack).xok()
	}
}

/// Define the default region for all descendants of
/// this entity.
#[derive(Debug, Clone, Component, Get)]
pub struct AwsStack {
	default_region: SmolStr,
}

impl Default for AwsStack {
	fn default() -> Self { Self::new(Self::DEFAULT_REGION) }
}

impl AwsStack {
	/// The default region used when no ancestor [`AwsStack`] is present.
	pub const DEFAULT_REGION: &'static str =
		crate::bindings::aws::region::DEFAULT;
	pub fn new(region: impl Into<SmolStr>) -> Self {
		Self {
			default_region: region.into(),
		}
	}
}
