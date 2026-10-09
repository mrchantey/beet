use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// `<DeployRoutes deploy={$up} destroy={$down}/>` — the standard IaC verb routes
/// as children: the two lifecycle verbs that run this stack's own groups
/// (`deploy` forward, `destroy` in reverse), the raw tofu lifecycle
/// (validate/plan/apply/show/list) and the artifact ledger's
/// rollback/rollforward.
///
/// Each verb resolves its stack by ancestry, so this carries no identity of its
/// own and hosting it is the whole declaration: author it under a `<Stack>` and
/// that stack has a lifecycle. That separation is the point, because a stage or
/// app name riding a TEMPLATE prop is absent from any binary that did not link
/// the template, which is exactly how a `shared` scope can go missing in a lean
/// build.
///
/// The two groups are the exception, and they are references rather than names:
/// what a stack's deploy DOES is the document's to say, and both halves of it
/// are declared beside the blocks they act on.
///
/// ```bsx
/// <Group bx:ref="up">
///     <TofuApply/>
/// </Group>
/// <Group bx:ref="down">
///     <StackTeardown/>
///     <TofuDestroy/>
/// </Group>
/// <DeployRoutes deploy={$up} destroy={$down}/>
/// ```
///
/// Both lifecycle verbs pass through the [`DeployGate`]: each runs plainly as
/// the repo's stored deploy credentials, refusing before its first step when
/// they cannot do what it is about to, and `--elevated` runs it with the human
/// factor instead.
///
/// Both are required and there is no compatibility mode: a stack that can be
/// brought up and not taken down is the thing this replaced. They are separate
/// declarations rather than one reversible group because a deploy is not a
/// teardown read backwards: a deploy provisions and probes, a teardown removes.
/// What IS the same list backwards is the teardown's own order, which is why
/// `destroy` runs its group in reverse.
#[template]
pub fn DeployRoutes(
	/// The group `deploy` runs, forward.
	#[prop(required)]
	deploy: Entity,
	/// The group `destroy` runs, in reverse. Authored in convergence order like
	/// every other group, so its first member is torn down last.
	#[prop(required)]
	destroy: Entity,
) -> impl Bundle {
	children![
		(
			PathPartial::new("deploy"),
			ParamsPartial::new::<ElevationParams>(),
			ExchangeGroup::default(),
			RunGroup::<Request, Response>::new(deploy),
			// the gate's overload in place of the group's own, declared rather
			// than required so it wins
			DeployGate,
			DeployGate::overload(),
		),
		(
			PathPartial::new("destroy"),
			ParamsPartial::new::<(DestroyParams, ElevationParams)>(),
			ExchangeGroup::forced_by("force"),
			RunGroup::<Request, Response>::reversed(destroy),
			DeployGate,
			DeployGate::overload(),
		),
		Validate,
		Plan,
		Apply,
		Show,
		List,
		Forget,
		RollState,
		Rollback,
		Rollforward
	]
}

/// Request params for the destroy route, surfaced in `--help` and read by
/// each step that honours `--force`.
#[derive(Reflect)]
pub(crate) struct DestroyParams {
	/// Keep going past a failing step, and destroy lock-free.
	pub force: bool,
}

impl terra::Project {
	/// Build a project from `caller`'s nearest ancestor [`Stack`], the
	/// resolution every stack verb starts from.
	pub async fn resolve(caller: &AsyncEntity) -> Result<Self> {
		let backend = Self::resolve_backend(caller).await?;
		caller
			.with_world(move |world, entity| {
				Self::resolve_in(world, entity, backend)
			})
			.await?
	}

	/// The launch's state backend, resolved: the async half of building a
	/// project, since the world pass that builds one is sync. An action that
	/// runs its own world pass resolves this first and hands it over, which is
	/// what makes holding a project proof that the discovery ran.
	pub async fn resolve_backend(
		caller: &AsyncEntity,
	) -> Result<ResolvedBackend> {
		caller
			.world()
			.with_resource::<Deployment, _>(|deployment| {
				deployment.backend().clone()
			})
			.await
			.resolve()
			.await
	}

	/// [`resolve`](Self::resolve) with the world in hand: the stack rendered
	/// against `backend`, and its secret store attached so a content variable
	/// resolves.
	pub fn resolve_in(
		world: &mut World,
		entity: Entity,
		backend: ResolvedBackend,
	) -> Result<Self> {
		let secrets = world.with_state::<StackQuery, _>(|stacks| {
			stacks.secret_store(entity)
		})?;
		RenderScope::render(world, entity)?
			.project(backend)?
			.with_secret_store(secrets)
			.xok()
	}
}

/// The [`ArtifactsClient`] of the stack's repo store, see
/// [`RepoStoreQuery::artifacts_client`].
async fn artifacts_client(caller: &AsyncEntity) -> Result<ArtifactsClient> {
	caller
		.with_state::<RepoStoreQuery, _>(|entity, repos| {
			repos.artifacts_client(entity)
		})
		.await?
}

/// Read the current ledger, point this launch's [`Deployment`] at it,
/// rebuild the config, and re-apply terraform.
async fn apply_with_current_ledger(caller: &AsyncEntity) -> Result<String> {
	let client = artifacts_client(caller).await?;
	let ledger = client
		.current_ledger()
		.await?
		.ok_or_else(|| bevyhow!("no current artifact ledger found"))?;

	// point this deploy at the target version
	let target_id = ledger.deploy_id;
	caller
		.world()
		.with_resource::<Deployment, _>(move |mut deployment| {
			deployment.update_from_ledger(&ledger);
		})
		.await;

	// rebuild and re-apply with the updated deploy_id
	let proj = terra::Project::resolve(caller).await?;
	info!("re-applying with deploy_id: {target_id}");
	proj.apply().await
}

/// Validate the stack configuration.
#[action(route = "validate")]
#[derive(Component)]
pub async fn Validate(cx: ActionContext) -> Result<String> {
	terra::Project::resolve(&cx.caller).await?.validate().await
}

/// Show the execution plan.
#[action(route = "plan")]
#[derive(Component)]
pub async fn Plan(cx: ActionContext) -> Result<String> {
	terra::Project::resolve(&cx.caller).await?.plan().await
}

/// Apply the execution plan.
#[action(route = "apply")]
#[derive(Component)]
pub async fn Apply(cx: ActionContext) -> Result<String> {
	terra::Project::resolve(&cx.caller).await?.apply().await
}

/// Show the current state.
#[action(route = "show")]
#[derive(Component)]
pub async fn Show(cx: ActionContext) -> Result<String> {
	terra::Project::resolve(&cx.caller).await?.show().await
}

/// List all resources in the state.
#[action(route = "list")]
#[derive(Component)]
pub async fn List(cx: ActionContext) -> Result<String> {
	terra::Project::resolve(&cx.caller).await?.list().await
}

/// Request params for [`Forget`], surfaced in `--help`.
#[derive(Reflect)]
struct ForgetParams {
	/// The state addresses to forget, comma separated, spelled as `list`
	/// prints them.
	resources: String,
}

/// Remove resources from the stack's state without touching them: the live
/// resource stays where it is and simply stops being managed (`tofu state
/// rm`). For a resource whose declaration moved out of the apply, whose next
/// plan would otherwise DESTROY what the config no longer renders; `tofu
/// import` is the way back.
#[action(route = "forget")]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<ForgetParams>())]
pub async fn Forget(cx: ActionContext<Request>) -> Result<String> {
	let resources = cx.input.parse_params::<ForgetParams>()?.resources;
	let resources = str_ext::csv(&resources).collect::<Vec<_>>();
	if resources.is_empty() {
		bevybail!(
			"`forget` names no resource: pass `--resources=<address>,..`"
		);
	}
	let project = terra::Project::resolve(&cx.caller).await?;
	let mut report = String::new();
	for resource in resources {
		report.push_str(&project.remove(resource).await?);
	}
	report.xok()
}

/// Request params for [`RollState`], surfaced in `--help`.
#[derive(Reflect)]
struct RollStateParams {
	/// The environment variable holding the passphrase the state is encrypted
	/// under now; the stack's `state_passphrase` variable holds the one it
	/// moves to. `<that variable>_OLD` unless given.
	retiring: Option<String>,
}

/// Re-encrypt the stack's state under its current passphrase, see
/// [`terra::Project::roll_state`]: the stack half of rolling
/// `TF_STATE_PASSPHRASE`, run once per stack after the document holds the
/// new value and `<variable>_OLD` the old. Also the one command that
/// encrypts a stack whose state is still plaintext.
#[action(route = "roll-state")]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<RollStateParams>())]
pub async fn RollState(cx: ActionContext<Request>) -> Result<String> {
	let retiring = cx.input.parse_params::<RollStateParams>()?.retiring;
	terra::Project::resolve(&cx.caller)
		.await?
		.roll_state(retiring.as_deref())
		.await
}

/// Request params for [`Rollback`], surfaced in `--help`.
#[derive(Reflect)]
struct RollbackParams {
	/// Number of versions to roll back, defaults to 1.
	count: Option<usize>,
}

/// Roll back to a previous artifact version, then re-apply infrastructure
/// with the rolled-back deploy_id.
#[action(route = "rollback")]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<RollbackParams>())]
pub async fn Rollback(cx: ActionContext<Request>) -> Result<String> {
	let count = cx
		.input
		.parse_params::<RollbackParams>()?
		.count
		.unwrap_or(1);
	let client = artifacts_client(&cx.caller).await?;
	let version = client.rollback(count).await?;
	info!("rolled back to version {version}");
	let result = apply_with_current_ledger(&cx.caller).await?;
	info!("{result}");
	format!("Rolled back to version {version} and re-applied").xok()
}

/// Roll forward to the latest artifact version, then re-apply infrastructure
/// with the latest deploy_id.
#[action(route = "rollforward")]
#[derive(Component)]
pub async fn Rollforward(cx: ActionContext) -> Result<String> {
	let client = artifacts_client(&cx.caller).await?;
	let version = client.rollforward().await?;
	info!("rolled forward to version {version}");
	let result = apply_with_current_ledger(&cx.caller).await?;
	info!("{result}");
	format!("Rolled forward to version {version} and re-applied").xok()
}

#[cfg(test)]
mod tests {
	use super::*;
	use beet_router::prelude::RouteTree;

	fn cli_world() -> World {
		(AsyncPlugin, RouterPlugin, InfraPlugin).into_world()
	}

	/// A stack with a lifecycle: two empty groups and the verbs that run them,
	/// built through `<DeployRoutes/>` exactly as an entry authors it.
	fn stack_with_verbs(world: &mut World) -> Entity {
		let deploy = world.spawn(Group).flush();
		let destroy = world.spawn(Group).flush();
		let root = world
			.spawn((Stack::new("test-app"), CliServer::default(), children![
				Router::with_defaults()
			]))
			.flush();
		let router = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity_mut(router)
			.insert_template(DeployRoutes {
				deploy: Some(deploy),
				destroy: Some(destroy),
			})
			.unwrap();
		world.flush();
		root
	}

	#[beet_core::test]
	fn routes_discoverable() {
		let mut world = cli_world();
		let root = stack_with_verbs(&mut world);
		let tree = RouteTree::of(&world, root).unwrap();
		// the lifecycle verbs
		tree.find(&["deploy"]).xpect_some();
		tree.find(&["destroy"]).xpect_some();
		// standard IaC routes
		tree.find(&["validate"]).xpect_some();
		tree.find(&["plan"]).xpect_some();
		tree.find(&["apply"]).xpect_some();
		tree.find(&["show"]).xpect_some();
		tree.find(&["list"]).xpect_some();
		tree.find(&["forget"]).xpect_some();
		tree.find(&["roll-state"]).xpect_some();
		// artifact routes
		tree.find(&["rollback"]).xpect_some();
		tree.find(&["rollforward"]).xpect_some();
	}

	#[beet_core::test]
	fn destroy_has_force_param() {
		let mut world = cli_world();
		let root = stack_with_verbs(&mut world);
		let tree = RouteTree::of(&world, root).unwrap();
		let destroy_node = tree.find(&["destroy"]).unwrap();
		world
			.entity(destroy_node.entity)
			.get::<ParamsPartial>()
			.xpect_some();
	}
}
