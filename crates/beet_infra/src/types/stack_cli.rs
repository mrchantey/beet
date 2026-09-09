use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

impl Stack {
	/// The standard IaC verb routes as children: the two lifecycle verbs that
	/// run this stack's own groups (`deploy` forward, `destroy` in reverse), the
	/// raw tofu lifecycle (validate/plan/apply/show/list) and the artifact
	/// ledger's rollback/rollforward.
	///
	/// Each resolves its stack by ancestry, so the bundle carries no identity of
	/// its own and hosting it is the whole declaration: spawn it under a
	/// `<Stack>` and that stack has a lifecycle.
	///
	/// `deploy` and `destroy` name the two groups, and both are required. They
	/// are separate declarations rather than one reversible group because a
	/// deploy is not a teardown read backwards: a deploy provisions and probes,
	/// a teardown removes. What IS the same list backwards is the teardown's own
	/// order, which is why `destroy` runs its group in reverse.
	pub fn verbs(deploy: Entity, destroy: Entity) -> impl Bundle {
		children![
			(
				PathPartial::new("deploy"),
				ExchangeGroup::default(),
				RunGroup::<Request, Response>::new(deploy),
			),
			(
				PathPartial::new("destroy"),
				ParamsPartial::new::<DestroyParams>(),
				ExchangeGroup::forced_by("force"),
				RunGroup::<Request, Response>::reversed(destroy),
			),
			Validate,
			Plan,
			Apply,
			Show,
			List,
			Rollback,
			Rollforward
		]
	}
}

/// Parameters for the destroy route.
#[derive(Reflect)]
struct DestroyParams {
	/// Keep going past a failing step, and destroy lock-free.
	force: bool,
}

impl terra::Project {
	/// Build a project from `caller`'s nearest ancestor [`Stack`], the
	/// resolution every stack verb starts from.
	pub async fn resolve(caller: &AsyncEntity) -> Result<Self> {
		caller
			.with_world(|world, entity| {
				RenderScope::render(world, entity)?.project()
			})
			.await?
	}
}

/// Build an [`ArtifactsClient`] from the nearest ancestor [`Stack`].
async fn artifacts_client(caller: &AsyncEntity) -> Result<ArtifactsClient> {
	caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.artifacts_client(entity)
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
	let proj = caller
		.with_world(|world, entity| {
			RenderScope::render(world, entity)?.project()
		})
		.await??;
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

/// Parameters for the rollback action.
#[derive(Reflect)]
struct RollbackParams {
	/// Number of versions to roll back, defaults to 1.
	count: Option<u32>,
}

/// Roll back to a previous artifact version, then re-apply infrastructure
/// with the rolled-back deploy_id.
#[action(route = "rollback")]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<RollbackParams>())]
pub async fn Rollback(cx: ActionContext<Request>) -> Result<String> {
	let count = cx
		.get_param("count")
		.and_then(|val| val.parse::<usize>().ok())
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

	/// A stack with a lifecycle: two empty groups and the verbs that run them.
	fn stack_with_verbs(world: &mut World) -> Entity {
		let deploy = world.spawn(Group).flush();
		let destroy = world.spawn(Group).flush();
		world
			.spawn((Stack::new("test-app"), CliServer::default(), children![(
				Router::with_defaults(),
				Stack::verbs(deploy, destroy)
			)]))
			.flush()
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
