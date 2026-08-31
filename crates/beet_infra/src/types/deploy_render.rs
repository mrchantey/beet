//! The on-demand schedule a deploy renders its tofu config through: each block
//! type contributes systems, ordering is expressed as system sets, and the
//! whole run reads and writes one [`RenderScope`].

use crate::prelude::*;
use beet_core::prelude::*;

/// The deploy render schedule, run on demand by [`RenderScope::render`].
///
/// Registered by [`InfraPlugin`] with [`DeployRenderSet::Declare`] before
/// [`DeployRenderSet::Render`], each block type adding its systems beside its
/// `register_type` under the same feature gates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, ScheduleLabel)]
pub struct DeployRender;

/// The two-stage ordering the render's one constraint lives in: a compute block
/// lowers the grants its *siblings* declared, so every declaration must land
/// before any render reads the pool.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum DeployRenderSet {
	/// Resource blocks state what a running process needs ([`AccessGrant`]) and
	/// the tofu [`Variable`]s they declare.
	Declare,
	/// Every block emits its resources into the config; computes lower the
	/// declared grants.
	Render,
}

/// One render run's world: the stack being rendered, the entities declared
/// under it, and everything the run accumulates. Inserted by
/// [`RenderScope::render`], read and written by every block system, then taken
/// out, so co-resident stacks cannot leak into each other's config.
#[derive(Resource)]
pub struct RenderScope {
	/// The entity carrying the stack's [`Stack`] declaration.
	root: Entity,
	/// The root's inclusive descendants in document order, the one traversal
	/// that decides what this stack declared ([`StackQuery::declared`]).
	declared: Vec<Entity>,
	stack: ResolvedStack,
	deployment: Deployment,
	/// Grant contributions, tagged by declaring entity so [`Self::access`]
	/// restores document order regardless of system order.
	grants: Vec<(Entity, AccessGrant)>,
	/// Tofu variable declarations, tagged like [`Self::grants`].
	variables: Vec<(Entity, Variable)>,
	config: terra::Config,
	/// Errors collected across the whole run rather than short-circuited, so
	/// one slow manual deploy reports every misconfiguration in one attempt.
	errors: Vec<BevyError>,
}

impl RenderScope {
	/// Render the stack `entity` belongs to: seed the scope, run
	/// [`DeployRender`], and take the scope back out.
	///
	/// The returned scope still carries any collected errors;
	/// [`finish`](Self::finish) is where they fail the call.
	pub fn render(world: &mut World, entity: Entity) -> Result<Self> {
		let (root, stack, deployment, declared) = world
			.with_state::<StackQuery, _>(|stacks| -> Result<_> {
				let (root, stack) = stacks.root(entity)?;
				(root, stack, stacks.deployment(), stacks.declared(entity)?)
					.xok()
			})?;
		let mut config = deployment.create_config(&stack);
		config.add_provider_config(
			&terra::Provider::AWS,
			&serde_json::json!({ "region": stack.region() }),
		)?;
		world.insert_resource(Self {
			root,
			declared,
			stack,
			deployment,
			grants: Vec::new(),
			variables: Vec::new(),
			config,
			errors: Vec::new(),
		});
		world.try_run_schedule(DeployRender).map_err(|_| {
			bevyhow!(
				"the DeployRender schedule is not registered; add InfraPlugin"
			)
		})?;
		world
			.remove_resource::<Self>()
			.ok_or_else(|| bevyhow!("RenderScope was removed mid-render"))
	}

	/// The rendered parts, or every collected error collapsed into one failure,
	/// decided BEFORE any tofu invocation: an action that planned a partial
	/// config is the silent under-provision this model exists to prevent.
	pub fn finish(self) -> Result<(ResolvedStack, Deployment, terra::Config)> {
		if self.errors.is_empty() {
			Ok((self.stack, self.deployment, self.config))
		} else {
			bevybail!(
				"deploy render failed with {} error(s):\n{}",
				self.errors.len(),
				self.errors
					.iter()
					.map(|err| format!("- {err}"))
					.collect::<Vec<_>>()
					.join("\n")
			)
		}
	}

	/// [`finish`](Self::finish) wrapped in the tofu driver that applies it,
	/// hence native-only.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn project(self) -> Result<terra::Project> {
		let (stack, deployment, config) = self.finish()?;
		terra::Project::new(stack, deployment, config).xok()
	}

	/// The entity carrying this run's [`Stack`].
	pub fn root(&self) -> Entity { self.root }

	/// The declared entities in document order, for a block system whose query
	/// is more than one component (pair with [`error`](Self::error)).
	pub fn declared(&self) -> &[Entity] { &self.declared }

	/// Collect an error against this run, failing [`finish`](Self::finish).
	pub fn error(&mut self, err: impl Into<BevyError>) {
		self.errors.push(err.into());
	}

	/// Resolve a consumer's relation `target` to the block component it names,
	/// for a render system emitting cross-block references: a missing relation,
	/// a target declared outside this stack, or a target carrying no `T` is an
	/// error naming `relation` and the consumer's `label` (collect it with
	/// [`error`](Self::error)), never a panic.
	pub fn related<'a, T: Component>(
		&self,
		query: &'a Query<&T>,
		target: Option<Entity>,
		relation: &str,
		label: &str,
	) -> Result<&'a T> {
		let type_name = core::any::type_name::<T>()
			.rsplit("::")
			.next()
			.unwrap_or_default();
		let Some(target) = target else {
			bevybail!(
				"'{label}' declares no `{relation}`: relate it to the \
				 declaration entity, ie `{{{relation}($name)}}`"
			);
		};
		if !self.declared.contains(&target) {
			bevybail!(
				"the `{relation}` of '{label}' targets {target}, which is not \
				 declared under this stack"
			);
		}
		query.get(target).map_err(|_| {
			bevyhow!(
				"the `{relation}` of '{label}' targets {target}, which carries \
				 no `{type_name}`"
			)
		})
	}

	/// The resolved identity every rendered name composes from.
	pub fn stack(&self) -> &ResolvedStack { &self.stack }

	/// This launch's deploy mechanics (its id, its artifacts bucket).
	pub fn deployment(&self) -> &Deployment { &self.deployment }

	/// Split borrows for a block body: the identity, the launch, and the config
	/// it emits into.
	pub fn ctx(&mut self) -> (&ResolvedStack, &Deployment, &mut terra::Config) {
		(&self.stack, &self.deployment, &mut self.config)
	}

	/// Contribute a grant from `entity`'s declaration.
	pub fn grant(&mut self, entity: Entity, grant: AccessGrant) {
		self.grants.push((entity, grant));
	}

	/// Contribute a tofu variable from `entity`'s declaration.
	pub fn variable(&mut self, entity: Entity, variable: Variable) {
		self.variables.push((entity, variable));
	}

	/// Every grant the stack's blocks declared, deduplicated in document order:
	/// declaration order is the deploy's order, so a policy renders identically
	/// across runs and a plan shows no spurious diff.
	pub fn access(&self) -> AccessGrants {
		self.sorted(&self.grants).xmap(AccessGrants::new)
	}

	/// Every tofu [`Variable`] the stack's blocks declared, in document order.
	pub fn variables(&self) -> Vec<Variable> { self.sorted(&self.variables) }

	/// Collect `contributions` in document order: contributions arrive grouped
	/// by block type (one system each), but consumers must see the order the
	/// document declared.
	fn sorted<T: Clone>(&self, contributions: &[(Entity, T)]) -> Vec<T> {
		let position = |entity: &Entity| {
			self.declared.iter().position(|decl| decl == entity)
		};
		let mut entries: Vec<_> = contributions.iter().enumerate().collect();
		entries.sort_by_key(|(index, (entity, _))| (position(entity), *index));
		entries
			.into_iter()
			.map(|(_, (_, value))| value.clone())
			.collect()
	}

	/// Run `func` for each declared entity carrying `T`, in document order,
	/// collecting each entity's error rather than short-circuiting the rest.
	pub fn render_each<T: Component, F>(
		&mut self,
		query: &Query<&T>,
		mut func: F,
	) where
		F: FnMut(&mut Self, Entity, &T) -> Result,
	{
		for entity in self.declared.clone() {
			if let Ok(block) = query.get(entity)
				&& let Err(err) = func(self, entity, block)
			{
				self.errors.push(err);
			}
		}
	}

	/// Pipe collector for a block system returning a whole-run [`Result`], ie
	/// `my_system.pipe(RenderScope::collect)`.
	pub fn collect(result: In<Result>, mut scope: ResMut<RenderScope>) {
		if let Err(err) = result.0 {
			scope.errors.push(err);
		}
	}
}

/// The one way a test renders blocks: through the same schedule and grant
/// pre-pass the deploy runs.
#[cfg(test)]
impl RenderScope {
	/// Render the blocks `func` spawns under a fresh local `<Stack>` root.
	/// Returns the scope with any collected errors still inside, so an error
	/// test asserts on them and a happy test calls [`finish`](Self::finish).
	pub(crate) fn test_render_stack(
		stack: Stack,
		func: impl FnOnce(&mut ChildSpawner),
	) -> (Self, crate::types::TestWorkDir) {
		let (deployment, dir) = Deployment::default_local();
		let mut world = InfraPlugin.into_world();
		world.insert_resource(deployment);
		world.init_resource::<PackageConfig>();
		let root = world.spawn(stack).with_children(func).id();
		let scope = Self::render(&mut world, root).unwrap();
		(scope, dir)
	}

	/// [`test_render_stack`](Self::test_render_stack) under the default test
	/// stack, mirroring [`ResolvedStack::default_local`].
	pub(crate) fn test_render(
		func: impl FnOnce(&mut ChildSpawner),
	) -> (Self, crate::types::TestWorkDir) {
		Self::test_render_stack(Stack::new("beet_infra"), func)
	}

	/// The terraform json the blocks `func` spawns render to, failing the test
	/// on any collected error.
	pub(crate) fn test_json(func: impl FnOnce(&mut ChildSpawner)) -> String {
		let (scope, _dir) = Self::test_render(func);
		scope.finish().unwrap().2.to_json_string().unwrap()
	}
}
