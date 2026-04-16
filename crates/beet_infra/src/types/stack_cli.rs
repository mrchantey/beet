use crate::prelude::*;
use beet_action::prelude::ActionContext;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

pub fn stack_cli() -> impl Bundle {
	(
		CliServer::default(),
		router(),
		OnSpawn::insert_child(Validate),
		OnSpawn::insert_child(Plan),
	)
}

async fn project(cx: ActionContext) -> Result<terra::Project> {
	cx.caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.build_project(entity)
		})
		.await
}

/// Validate the stack
#[action(route = "validate")]
#[derive(Component, Reflect)]
#[reflect(Component)]
async fn Validate(cx: ActionContext) -> Result<String> {
	project(cx).await?.validate().await
}
/// Plan the stack
#[action(route = "plan")]
#[derive(Component, Reflect)]
#[reflect(Component)]
async fn Plan(cx: ActionContext) -> Result<String> {
	project(cx).await?.plan().await
}
