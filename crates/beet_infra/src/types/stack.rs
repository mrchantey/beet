use crate::prelude::*;
use crate::terra::Project;
use beet_core::prelude::*;

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
			stage: "dev".into(),
			prod_stage: "prod".into(),
			params: default(),
			backend: S3Backend::default().into(),
		}
	}

	#[cfg(test)]
	pub fn default_local() -> Self {
		Self {
			backend: LocalBackend::default().into(),
			..default()
		}
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
	/// Initialize a config with the corresponding backend
	pub fn create_config(&self) -> terra::Config {
		terra::Config::default().with_backend(self.backend())
	}
}


#[derive(SystemParam)]
pub struct StackQuery<'w, 's> {
	stacks: AncestorQuery<'w, 's, (Entity, &'static Stack)>,
	blocks: Query<'w, 's, (EntityRef<'static>, &'static ErasedBlock)>,
	children: Query<'w, 's, &'static Children>,
	// ancestors: Query<'w, 's, &'static ChildOf>,
}

impl<'w, 's> StackQuery<'w, 's> {
	/// Finds the stack in ancestors and
	/// builds a config of all block descendents
	pub fn build_project(&self, entity: Entity) -> Result<terra::Project> {
		let (root, stack) = self.stacks.get(entity)?;
		let mut config = stack.create_config();
		for (child, block) in self
			.children
			.iter_descendants_inclusive(root)
			.filter_map(|child| self.blocks.get(child).ok())
		{
			block.apply_to_config(&child, stack, &mut config)?;
		}
		Ok(Project::new(stack.work_directory().into_abs(), config))
	}
}
