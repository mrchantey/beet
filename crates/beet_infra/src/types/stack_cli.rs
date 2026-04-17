use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

pub fn stack_cli() -> impl Bundle {
	(
		CliServer::default(),
		router(),
		OnSpawn::insert_child(Validate),
		OnSpawn::insert_child(Plan),
		OnSpawn::insert_child(Apply),
		OnSpawn::insert_child(Show),
		OnSpawn::insert_child(List),
		OnSpawn::insert_child(Destroy),
		OnSpawn::insert_child(Rollback),
		OnSpawn::insert_child(Rollforward),
	)
}

/// Build a [`terra::Project`] from the nearest ancestor [`Stack`].
async fn project(caller: &AsyncEntity) -> Result<terra::Project> {
	caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.build_project(entity)
		})
		.await
}

/// Build an [`ArtifactsClient`] from the nearest ancestor [`Stack`].
async fn artifacts_client(caller: &AsyncEntity) -> Result<ArtifactsClient> {
	caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.artifacts_client(entity)
		})
		.await
}

/// Read the current ledger and stack, then apply with artifact vars.
async fn apply_with_current_ledger(caller: &AsyncEntity) -> Result<String> {
	let (proj, stack) = caller
		.with_state::<(StackQuery, AncestorQuery<&Stack>), _>(
			|entity, (stack_query, anc_stack)| -> Result<_> {
				let project = stack_query.build_project(entity)?;
				let stack = anc_stack.get(entity)?.clone();
				(project, stack).xok()
			},
		)
		.await?;
	let client = stack.artifacts_client();
	let ledger = client.current_ledger().await?.ok_or_else(|| {
		bevyhow!("no current artifact ledger found")
	})?;
	let vars = ledger.build_vars(&stack.artifact_bucket_name());
	info!("re-applying with {} artifact variables", vars.len());
	proj.apply_with_vars(&vars).await
}

/// Validate the stack configuration.
#[action(route = "validate")]
#[derive(Component)]
async fn Validate(cx: ActionContext) -> Result<String> {
	project(&cx.caller).await?.validate().await
}

/// Show the execution plan.
#[action(route = "plan")]
#[derive(Component)]
async fn Plan(cx: ActionContext) -> Result<String> {
	project(&cx.caller).await?.plan().await
}

/// Apply the execution plan.
#[action(route = "apply")]
#[derive(Component)]
async fn Apply(cx: ActionContext) -> Result<String> {
	project(&cx.caller).await?.apply().await
}

/// Show the current state.
#[action(route = "show")]
#[derive(Component)]
async fn Show(cx: ActionContext) -> Result<String> {
	project(&cx.caller).await?.show().await
}

/// List all resources in the state.
#[action(route = "list")]
#[derive(Component)]
async fn List(cx: ActionContext) -> Result<String> {
	project(&cx.caller).await?.list().await
}

/// Parameters for the destroy action.
#[derive(Reflect)]
struct DestroyParams {
	/// Skip confirmation and force destroy.
	force: bool,
}

/// Destroy infrastructure, with optional force flag.
#[action(route = "destroy")]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<DestroyParams>())]
async fn Destroy(cx: ActionContext<Request>) -> Result<String> {
	let force = cx.has_param("force");
	let proj = project(&cx.caller).await?;
	if force {
		proj.force_destroy().await;
		"Force destroy complete".to_string().xok()
	} else {
		proj.destroy().await?;
		"Destroy complete".to_string().xok()
	}
}

/// Parameters for the rollback action.
#[derive(Reflect)]
struct RollbackParams {
	/// Number of versions to roll back, defaults to 1.
	count: Option<u32>,
}

/// Roll back to a previous artifact version, then re-apply infrastructure
/// with the rolled-back artifact variables.
#[action(route = "rollback")]
#[derive(Component)]
#[require(ParamsPartial = ParamsPartial::new::<RollbackParams>())]
async fn Rollback(cx: ActionContext<Request>) -> Result<String> {
	let count = cx
		.get_param("count")
		.and_then(|val| val.parse::<usize>().ok())
		.unwrap_or(1);
	let client = artifacts_client(&cx.caller).await?;
	let version = client.rollback(count).await?;
	info!("rolled back to version {version}");
	// re-apply infrastructure with the rolled-back artifact vars
	apply_with_current_ledger(&cx.caller).await?;
	format!("Rolled back to version {version} and re-applied").xok()
}

/// Roll forward to the latest artifact version, then re-apply infrastructure
/// with the latest artifact variables.
#[action(route = "rollforward")]
#[derive(Component)]
async fn Rollforward(cx: ActionContext) -> Result<String> {
	let client = artifacts_client(&cx.caller).await?;
	let version = client.rollforward().await?;
	info!("rolled forward to version {version}");
	// re-apply infrastructure with the latest artifact vars
	apply_with_current_ledger(&cx.caller).await?;
	format!("Rolled forward to version {version} and re-applied").xok()
}


#[cfg(test)]
mod tests {
	use super::*;
	use beet_router::prelude::RouteTree;

	fn cli_world() -> World {
		(AsyncPlugin, RouterPlugin, InfraPlugin).into_world()
	}

	#[test]
	fn routes_discoverable() {
		let mut world = cli_world();
		let root = world
			.spawn((
				Stack::new("test-app").with_backend(LocalBackend::default()),
				stack_cli(),
			))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		// standard IaC routes
		tree.find(&["validate"]).xpect_some();
		tree.find(&["plan"]).xpect_some();
		tree.find(&["apply"]).xpect_some();
		tree.find(&["show"]).xpect_some();
		tree.find(&["list"]).xpect_some();
		tree.find(&["destroy"]).xpect_some();
		// artifact routes
		tree.find(&["rollback"]).xpect_some();
		tree.find(&["rollforward"]).xpect_some();
	}

	#[test]
	fn destroy_has_force_param() {
		let mut world = cli_world();
		let root = world
			.spawn((
				Stack::new("test-app").with_backend(LocalBackend::default()),
				stack_cli(),
			))
			.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		let destroy_node = tree.find(&["destroy"]).unwrap();
		// verify the entity has a ParamsPartial component
		world
			.entity(destroy_node.entity)
			.get::<ParamsPartial>()
			.xpect_some();
	}
}
