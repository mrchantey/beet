//! The hierarchical source of truth for cloud resource identity: the authored
//! [`Stack`], the total [`ResolvedStack`] it resolves to, and the traversal
//! every deploy step reaches them through.

use crate::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::terra::Project;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The DECLARATION of a resource scope: which app, which stage, which region
/// the resources beneath it belong to, each field optional and each falling back
/// to this process's own answer.
///
/// Markup-authorable and registered in every native build, so `<Stack/>` bare
/// works everywhere and `<Stack stage="shared"/>` overrides exactly one field.
/// It is deliberately NOT the thing a name composes from: that is
/// [`ResolvedStack`], which [`resolve`](Self::resolve) produces and which has no
/// optional fields, so a half-answered identity cannot reach a resource name.
#[derive(Debug, Default, Clone, PartialEq, Eq, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Stack {
	/// The app identity, else the process [`PackageConfig`]'s. Overriding it is
	/// for a multi-app entry; an app naming itself twice is exactly the drift
	/// this composition exists to prevent.
	#[set_with(unwrap_option, into)]
	app_name: Option<SmolStr>,
	/// The deployment stage namespacing every resource, else this launch's
	/// (`--stage=<x>`, else `BEET_STAGE`, else `dev`). A resource not owned by
	/// any deploy stage declares its own, ie `<Stack stage="shared"/>`.
	#[set_with(unwrap_option, into)]
	stage: Option<SmolStr>,
	/// The aws region every resource beneath this stack lives in, else
	/// `AWS_REGION`, else the crate default.
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
	/// Additional parameters, some of which may be required by a config
	/// generator.
	#[reflect(ignore)]
	params: MultiMap<SmolStr, SmolStr>,
}

impl Stack {
	/// A stack naming `app_name` explicitly, the code counterpart of
	/// `<Stack app_name=".."/>`.
	pub fn new(app_name: impl Into<SmolStr>) -> Self {
		Self::default().with_app_name(app_name)
	}

	/// Fill every unset field from this process's answer, the ONE place a stack
	/// consults [`PackageConfig`], [`BootstrapConfig`] or `AWS_REGION`.
	///
	/// Total by construction: `package` is required and its
	/// [`app_name`](PackageConfig::app_name) always set, so there is no
	/// half-resolved outcome and nothing downstream re-asks the question.
	pub fn resolve(&self, package: &PackageConfig) -> ResolvedStack {
		ResolvedStack {
			app_name: self
				.app_name
				.clone()
				.unwrap_or_else(|| package.app_name().into()),
			stage: self
				.stage
				.clone()
				.unwrap_or_else(|| BootstrapConfig::get().stage.clone()),
			region: self.region.clone().unwrap_or_else(|| {
				env_ext::var("AWS_REGION")
					.ok()
					.map(SmolStr::from)
					.unwrap_or_else(|| {
						crate::bindings::aws::region::DEFAULT.into()
					})
			}),
			params: self.params.clone(),
		}
	}
}

/// A [`Stack`] with every field answered: the app identity, stage and region
/// that turn a resource *label* into a resource *name*, ie the `beet-site` +
/// `prod` behind `beet-site--prod--analytics`.
///
/// A declaration carries only its label, and BOTH meanings of that declaration
/// compose the name here: the deploy that creates the resource and the runtime
/// that reads or writes it. One composition, so the two cannot drift, and it
/// takes a RESOLVED stack so a name can never compose from a scope that was
/// never resolved.
///
/// Only [`Stack::resolve`] builds one, which makes that function the single
/// in-world reader of `AWS_REGION`: every store is handed a resolved region
/// rather than reaching for one itself.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith)]
pub struct ResolvedStack {
	app_name: SmolStr,
	stage: SmolStr,
	region: SmolStr,
	params: MultiMap<SmolStr, SmolStr>,
}

impl ResolvedStack {
	/// Whether this stack deploys the [production
	/// stage](BootstrapConfig::PROD_STAGE), which often receives special
	/// treatment like bucket locking and no subdomain.
	pub fn is_production(&self) -> bool {
		self.stage == BootstrapConfig::PROD_STAGE
	}

	/// The identifier a resource label composes to in this stack, the single
	/// definition of the `app--stage--label` convention.
	pub fn resource_ident(&self, label: impl Into<SmolStr>) -> terra::Ident {
		terra::Ident::new(self.app_name.clone(), self.stage.clone(), label)
	}

	/// The provider-facing resource name, ie `beet-site--prod--analytics`.
	pub fn resource_name(&self, label: impl Into<SmolStr>) -> String {
		self.resource_ident(label).primary_identifier().to_string()
	}

	/// The tofu config `blocks` build in this stack: the provider region, then
	/// every block's resources emitted with the [`AccessGrants`] the whole set
	/// declared.
	///
	/// The one definition of what a stack's config *is*, so the ECS traversal
	/// ([`StackQuery::build_config`]) and a caller holding blocks directly (a
	/// test, a wasm consumer authoring a stack in Rust) cannot drift on the
	/// grant pre-pass, which is easy to omit and silently under-grants the
	/// deployed identity.
	pub fn build_config<'a>(
		&self,
		deployment: &Deployment,
		blocks: impl IntoIterator<Item = (EntityRef<'a>, &'a dyn Block)> + Clone,
	) -> Result<terra::Config> {
		let mut config = deployment.create_config(self);
		config.add_provider_config(
			&terra::Provider::AWS,
			&serde_json::json!({ "region": self.region }),
		)?;
		// a pre-pass, since a compute block lowers the grants its *siblings*
		// declared and the emit order is otherwise arbitrary.
		let access = blocks
			.clone()
			.into_iter()
			.flat_map(|(_, block)| block.runtime_access(self))
			.collect::<Vec<_>>()
			.xmap(AccessGrants::new);
		for (entity, block) in blocks {
			block.apply_to_config(
				&entity,
				self,
				deployment,
				&access,
				&mut config,
			)?;
		}
		config.xok()
	}

	/// A resolved stack plus the launch that deploys it locally: a local state
	/// backend and a temporary work directory removed on drop.
	#[cfg(test)]
	pub fn default_local() -> (Self, Deployment, crate::types::TestWorkDir) {
		let (deployment, dir) = Deployment::default_local();
		(
			Stack::new("beet_infra").resolve(&PackageConfig::default()),
			deployment,
			dir,
		)
	}
}

/// Resolves the [`Stack`] an entity belongs to, and the deploy traversal that
/// starts from it.
#[derive(SystemParam)]
pub struct StackQuery<'w, 's> {
	stacks: AncestorQuery<'w, 's, (Entity, &'static Stack)>,
	all_stacks: Query<'w, 's, (Entity, &'static Stack)>,
	blocks: Query<'w, 's, (EntityRef<'static>, &'static ErasedBlock)>,
	children: Query<'w, 's, &'static Children>,
	stores: Query<'w, 's, &'static BlobStore>,
	/// The process app identity, which [`BootstrapPlugin`] inserts at build time
	/// and [`InfraPlugin`] therefore guarantees, so resolution is total.
	package: Res<'w, PackageConfig>,
	deployment: Option<Res<'w, Deployment>>,
}

impl<'w, 's> StackQuery<'w, 's> {
	/// The [`ResolvedStack`] `entity` composes its resource names against: the
	/// nearest ancestor's, else the process default. A declaration outside every
	/// stack is not an error, it simply belongs to no deploy's config and
	/// resolves the names the process itself would.
	pub fn resolve(&self, entity: Entity) -> ResolvedStack {
		self.stack(entity)
			.cloned()
			.unwrap_or_default()
			.resolve(&self.package)
	}

	/// The nearest ancestor [`Stack`] as authored, if any. A declaration made
	/// purely for its runtime meaning has none.
	pub fn stack(&self, entity: Entity) -> Option<&Stack> {
		self.stacks.get(entity).ok().map(|(_, stack)| stack)
	}

	/// This launch's [`Deployment`], which [`InfraPlugin`] inits, else the
	/// derived default for a world that has no infra plugin.
	pub fn deployment(&self) -> Deployment {
		self.deployment
			.as_deref()
			.cloned()
			.unwrap_or_else(Deployment::default)
	}

	/// The entity carrying the nearest ancestor [`Stack`], and that stack
	/// resolved: the root every block, artifact and verb under one deploy
	/// resolves against.
	pub fn root(&self, entity: Entity) -> Result<(Entity, ResolvedStack)> {
		let (root, stack) = self.stacks.get(entity)?;
		Ok((root, stack.resolve(&self.package)))
	}

	/// Every entity declared under `entity`'s stack: its root's descendants
	/// (inclusive), plus any UNSCOPED block (see [`Self::unscoped_blocks`]) when
	/// this stack is the one that adopts them. The one traversal a deploy step
	/// uses to find what was declared alongside it.
	pub fn declared(&self, entity: Entity) -> Result<Vec<Entity>> {
		let (root, _) = self.root(entity)?;
		let mut declared = self
			.children
			.iter_descendants_inclusive(root)
			.collect::<Vec<_>>();
		declared.extend(self.adopted_blocks(root)?);
		declared.xok()
	}

	/// Blocks declared outside any [`Stack`], ie an application-level resource
	/// declaration whose reason to exist is its runtime meaning (the analytics
	/// table a router records to). They are real resources and something must
	/// provision them, so a deploy adopts them.
	fn unscoped_blocks(&self) -> Vec<Entity> {
		self.blocks
			.iter()
			.map(|(entity_ref, _)| entity_ref.id())
			.filter(|entity| self.stacks.get(*entity).is_err())
			.collect()
	}

	/// The unscoped blocks this stack adopts: none unless it is THE host for
	/// this process's stage, so a stage override (the shared assets host, which
	/// deploys nothing the app runs on) never quietly provisions the app's
	/// resources into the wrong stack.
	fn adopted_blocks(&self, root: Entity) -> Result<Vec<Entity>> {
		let unscoped = self.unscoped_blocks();
		if unscoped.is_empty() {
			return Ok(Vec::new());
		}
		let process_stage = &BootstrapConfig::get().stage;
		let hosts = self
			.all_stacks
			.iter()
			.filter(|(_, stack)| {
				stack.resolve(&self.package).stage() == process_stage
			})
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		// naming the blocks, since the fix is either to scope them under a host
		// or to declare the host they belong to.
		if hosts.len() != 1 {
			bevybail!(
				"{} block(s) are declared outside any deploy host, and {} hosts carry the process stage '{process_stage}' (exactly one must): {unscoped:?}",
				unscoped.len(),
				hosts.len()
			);
		}
		match hosts[0] == root {
			true => unscoped,
			false => Vec::new(),
		}
		.xok()
	}

	/// Finds the stack in ancestors and builds a config of all block
	/// descendants, with the AWS provider region resolved from that stack so the
	/// tofu config and the Rust SDK cannot disagree.
	///
	/// This is the whole definition step, and it is target-agnostic: a wasm
	/// consumer authors blocks and builds the config here, then serializes it for
	/// a host that can apply it (see [`build_project`](Self::build_project),
	/// which is the same config wrapped in the native tofu driver).
	pub fn build_config(
		&self,
		entity: Entity,
	) -> Result<(ResolvedStack, Deployment, terra::Config)> {
		let (_, stack) = self.root(entity)?;
		let deployment = self.deployment();
		let blocks = self
			.declared(entity)?
			.into_iter()
			.filter_map(|child| self.blocks.get(child).ok())
			.map(|(entity, block)| (entity, &**block))
			.collect::<Vec<_>>();
		let config = stack.build_config(&deployment, blocks)?;
		Ok((stack, deployment, config))
	}

	/// [`build_config`](Self::build_config) wrapped in the tofu driver that
	/// applies it, hence native-only.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn build_project(&self, entity: Entity) -> Result<terra::Project> {
		let (stack, deployment, config) = self.build_config(entity)?;
		Ok(Project::new(stack, deployment, config))
	}

	/// Create an artifacts client for the stack at the given entity.
	pub fn artifacts_client(&self, entity: Entity) -> Result<ArtifactsClient> {
		let (_, stack) = self.root(entity)?;
		self.deployment().artifacts_client(&stack).xok()
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

	/// A stack resolved against an app that names itself.
	fn resolved(stack: Stack) -> ResolvedStack {
		stack.resolve(&PackageConfig {
			app_name: "beet-site".into(),
			..default()
		})
	}

	/// The composition is the one the live stacks already resolve; renaming a
	/// deployed resource is a production incident, so these strings are pinned.
	#[beet_core::test]
	fn composes_the_live_names() {
		let prod = resolved(Stack::default().with_stage("prod"));
		prod.resource_name("analytics")
			.xpect_eq("beet-site--prod--analytics");
		prod.resource_name("app").xpect_eq("beet-site--prod--app");
		resolved(Stack::default().with_stage("dev"))
			.resource_name("artifacts")
			.xpect_eq("beet-site--dev--artifacts");
		resolved(Stack::default().with_stage("shared"))
			.resource_name("assets")
			.xpect_eq("beet-site--shared--assets");
		resolved(Stack::new("beet").with_stage("shared"))
			.resource_name("assets")
			.xpect_eq("beet--shared--assets");
	}

	/// Every unset field falls back to this process's answer: the app name to
	/// the [`PackageConfig`], and the stage to the launch (the beet test runner
	/// passes no `--stage`/`BEET_STAGE`, so it is `dev` and not production).
	#[beet_core::test]
	fn resolves_the_process_defaults() {
		let stack = resolved(Stack::default());
		stack.app_name().as_str().xpect_eq("beet-site");
		stack.stage().as_str().xpect_eq("dev");
		stack.is_production().xpect_false();
	}

	/// An app that declared no name still composes a name rather than an empty
	/// segment, and both meanings of a declaration compose the SAME one, which
	/// is what makes a generic fallback safe (the live incident it guards was
	/// two INDEPENDENT derivations, not the fallback itself).
	#[beet_core::test]
	fn an_unnamed_app_resolves_the_generic_name() {
		Stack::default()
			.resolve(&PackageConfig::default())
			.resource_name("analytics")
			.xpect_eq("beet-app--dev--analytics");
	}

	/// The `prod` stage (what `--stage=prod` resolves to) marks production,
	/// flipping the stage-aware paths (eg the beet-site apex dns).
	#[beet_core::test]
	fn prod_stage_is_production() {
		resolved(Stack::default().with_stage("prod"))
			.is_production()
			.xpect_true();
	}

	/// An authored field wins over the process answer, and resolution never
	/// clobbers it.
	#[beet_core::test]
	fn authored_fields_win() {
		let stack = resolved(
			Stack::new("other-app")
				.with_stage("shared")
				.with_region("eu-west-1"),
		);
		stack.app_name().as_str().xpect_eq("other-app");
		stack.stage().as_str().xpect_eq("shared");
		stack.region().as_str().xpect_eq("eu-west-1");
	}

	/// The region an all-default stack resolves, unchanged from the per-block
	/// `region` fields this phase turned into overrides: those defaulted to this
	/// same constant, so the rendered tofu value must not move. A changed region
	/// REPLACES every physical resource.
	#[beet_core::test]
	fn default_region_is_the_pinned_constant() {
		crate::bindings::aws::region::DEFAULT.xpect_eq("us-west-2");
		// `AWS_REGION` is the documented first fallback, so the constant only
		// governs a process carrying none.
		if env_ext::var("AWS_REGION").is_err() {
			resolved(Stack::default())
				.region()
				.as_str()
				.xpect_eq("us-west-2");
		}
	}

	/// Two stacks sharing one launch compose distinct state paths, so a `shared`
	/// deploy can never overwrite the stage deploy's state. The keys are the ones
	/// the live backends already hold (the suffix kebab-cases with every other
	/// segment), so they are pinned too.
	#[beet_core::test]
	fn stacks_share_a_launch_and_split_their_state() {
		let (deployment, _dir) = Deployment::default_local();
		let stage = resolved(Stack::default().with_stage("dev"));
		let shared = resolved(Stack::default().with_stage("shared"));
		deployment
			.backend_path(&stage)
			.to_string()
			.xpect_eq("beet-site--dev--tofu-tfstate");
		deployment
			.backend_path(&shared)
			.to_string()
			.xpect_eq("beet-site--shared--tofu-tfstate");
		deployment
			.artifact_bucket_name(&stage)
			.xpect_eq("beet-site--dev--artifacts");
	}
}
