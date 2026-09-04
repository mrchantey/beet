//! The on-demand schedule a deploy renders its tofu config through: each block
//! type contributes systems, ordering is expressed as system sets, and every
//! system reaches up to the [`RenderScope`] on its stack root and writes to it
//! directly.

use crate::prelude::*;
use beet_core::prelude::*;

/// The deploy render schedule, run on demand by [`RenderScope::render`].
///
/// Registered by [`InfraPlugin`] with [`DeployRenderSet::Declare`] before
/// [`DeployRenderSet::Render`], each block type adding its systems beside its
/// `register_type` under the same feature gates. Most blocks ride the generic
/// [`declare`] / [`render`] systems; a block with cross-entity inputs (a
/// relation, an artifact, the grant pool) registers its own render system in
/// the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, ScheduleLabel)]
pub struct DeployRender;

/// The two-stage ordering the render's one constraint lives in: a compute block
/// lowers the grants its *siblings* declared, so every declaration must land
/// before any render reads the pool. Order within the pool is no constraint at
/// all: [`AccessGrants`] is a sorted set by construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub enum DeployRenderSet {
	/// Blocks state what a running process needs ([`AccessGrant`]) and the
	/// tofu [`Variable`]s they declare.
	Declare,
	/// Blocks emit their resources into the config; computes lower the
	/// declared pool.
	Render,
}

/// One render run, as a component on the stack ROOT being rendered: the
/// resolved identity, the pools the run accumulates, and the config it emits
/// into. Inserted by [`RenderScope::render`], reached up to by every block
/// system ([`AncestorQuery::get_mut`]), and taken back out by the seam, which
/// is the whole reset: the scope is the run's only transient state.
///
/// Sitting on the root rather than in a resource is what scopes it: a block
/// contributes to the nearest scope above it and to no other, so co-resident
/// stacks cannot leak into each other's config, and several scopes may render
/// in one schedule run ([`RenderScope::render_all`]).
#[derive(Component)]
pub struct RenderScope {
	stack: ResolvedStack,
	deployment: Deployment,
	grants: Vec<AccessGrant>,
	variables: Vec<Variable>,
	config: terra::Config,
	/// Errors collected across the whole run rather than short-circuited, so
	/// one slow manual deploy reports every misconfiguration in one attempt.
	errors: Vec<BevyError>,
}

impl RenderScope {
	/// Render the stack `entity` belongs to: seed a scope on its root, run
	/// [`DeployRender`], and take the scope back out.
	///
	/// The returned scope still carries any collected errors;
	/// [`finish`](Self::finish) is where they fail the call.
	pub fn render(world: &mut World, entity: Entity) -> Result<Self> {
		let root = world.with_state::<StackQuery, _>(|stacks| {
			stacks.root(entity).map(|(root, _)| root)
		})?;
		Self::render_roots(world, vec![root])?
			.pop()
			.ok_or_else(|| bevyhow!("RenderScope was removed mid-render"))
	}

	/// Render every declared stack in one schedule run, in the order their
	/// roots spawn. Under several scopes a block contributes to the NEAREST
	/// one above it, so a stack nested inside another renders its own blocks;
	/// [`render`](Self::render) of the outer stack (one scope in the world)
	/// keeps the whole-subtree semantics.
	pub fn render_all(world: &mut World) -> Result<Vec<Self>> {
		let roots =
			world.with_state::<Query<Entity, With<Stack>>, _>(|stacks| {
				stacks.iter().collect::<Vec<_>>()
			});
		Self::render_roots(world, roots)
	}

	/// Seed a scope on each of `roots`, run the schedule once, take each scope
	/// back out. That take IS the reset: nothing else persists a run.
	fn render_roots(
		world: &mut World,
		roots: Vec<Entity>,
	) -> Result<Vec<Self>> {
		for root in roots.iter() {
			let scope = world.with_state::<StackQuery, _>(|stacks| {
				Self::new(stacks.resolve(*root), stacks.deployment())
			})?;
			world.entity_mut(*root).insert(scope);
		}
		world.try_run_schedule(DeployRender).map_err(|_| {
			bevyhow!(
				"the DeployRender schedule is not registered; add InfraPlugin"
			)
		})?;
		roots
			.iter()
			.map(|root| {
				world.entity_mut(*root).take::<Self>().ok_or_else(|| {
					bevyhow!("RenderScope was removed mid-render")
				})
			})
			.collect()
	}

	/// A seeded scope: the backend and encryption this launch deploys with,
	/// and the provider region the stack resolves.
	fn new(stack: ResolvedStack, deployment: Deployment) -> Result<Self> {
		let mut config = deployment.create_config(&stack);
		config.add_provider_config(
			&terra::Provider::AWS,
			&serde_json::json!({ "region": stack.region() }),
		)?;
		Ok(Self {
			stack,
			deployment,
			grants: Vec::new(),
			variables: Vec::new(),
			config,
			errors: Vec::new(),
		})
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

	/// The resolved identity every rendered name composes from.
	pub fn stack(&self) -> &ResolvedStack { &self.stack }

	/// This launch's deploy mechanics (its id, its artifacts bucket).
	pub fn deployment(&self) -> &Deployment { &self.deployment }

	/// Split borrows for a block body: the identity, the launch, and the config
	/// it emits into.
	pub fn ctx(&mut self) -> (&ResolvedStack, &Deployment, &mut terra::Config) {
		(&self.stack, &self.deployment, &mut self.config)
	}

	/// Contribute a block's declarations to the pools.
	pub fn declare(
		&mut self,
		grants: Vec<AccessGrant>,
		variables: Vec<Variable>,
	) {
		self.grants.extend(grants);
		self.variables.extend(variables);
	}

	/// Every grant the stack's blocks declared, as the sorted set it is:
	/// contribution order can never matter, so a reordering of declarations
	/// never diffs a rendered policy.
	pub fn access(&self) -> AccessGrants {
		AccessGrants::new(self.grants.clone())
	}

	/// Every tofu [`Variable`] the stack's blocks declared.
	pub fn variables(&self) -> Vec<Variable> { self.variables.clone() }

	/// Collect an error against this run, failing [`finish`](Self::finish).
	pub fn error(&mut self, err: impl Into<BevyError>) {
		self.errors.push(err.into());
	}
}

/// The generic Declare-set system: contribute each declared `T`'s grants and
/// variables to the scope above it. A block outside every rendering scope is
/// simply not being rendered.
pub(crate) fn declare<T: Block>(
	mut scopes: AncestorQuery<&mut RenderScope>,
	blocks: Query<(Entity, &T)>,
) {
	for (entity, block) in blocks.iter() {
		let Ok(mut scope) = scopes.get_mut(entity) else {
			continue;
		};
		let grants = block.grants(scope.stack());
		let variables = block.variables();
		scope.declare(grants, variables);
	}
}

/// The generic Render-set system for a simple block: emit each declared `T`
/// into the scope above it, collecting per-entity errors attributed to the
/// block rather than short-circuiting the rest.
pub(crate) fn render<T: EmitBlock>(
	mut scopes: AncestorQuery<&mut RenderScope>,
	blocks: Query<(Entity, &T)>,
) {
	for (entity, block) in blocks.iter() {
		let Ok(mut scope) = scopes.get_mut(entity) else {
			continue;
		};
		let (stack, deployment, config) = scope.ctx();
		if let Err(err) = block.emit(stack, deployment, config) {
			let err = bevyhow!(
				"{} '{}': {err}",
				type_ext::short_name::<T>(),
				block.label()
			);
			scope.error(err);
		}
	}
}

/// Resolve a consumer's relation `target` to the block component it names, for
/// a bespoke render system emitting cross-block references: a missing relation,
/// a target outside the consumer's scope, or a target carrying no `T` is an
/// error naming `relation` and the consumer's `label` (collect it with
/// [`RenderScope::error`]), never a panic.
pub(crate) fn related<'a, T: Component>(
	scopes: &AncestorQuery<&mut RenderScope>,
	consumer: Entity,
	blocks: &'a Query<&T>,
	target: Option<Entity>,
	relation: &str,
	label: &str,
) -> Result<&'a T> {
	let Some(target) = target else {
		bevybail!(
			"'{label}' declares no `{relation}`: relate it to the declaration \
			 entity, ie `{{{relation}($name)}}`"
		);
	};
	// in the consumer's scope = both resolve the same scope root
	let in_scope = scopes
		.get_entity(consumer)
		.ok()
		.zip(scopes.get_entity(target).ok())
		.is_some_and(|(consumer_root, target_root)| {
			consumer_root == target_root
		});
	if !in_scope {
		bevybail!(
			"the `{relation}` of '{label}' targets {target}, which is not \
			 declared under this stack"
		);
	}
	blocks.get(target).map_err(|_| {
		bevyhow!(
			"the `{relation}` of '{label}' targets {target}, which carries \
			 no `{}`",
			type_ext::short_name::<T>()
		)
	})
}

/// The one way a test renders blocks: through the same schedule and grant
/// pool the deploy runs.
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
