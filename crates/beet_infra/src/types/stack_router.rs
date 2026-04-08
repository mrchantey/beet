use crate::prelude::*;
use beet_core::prelude::*;
use beet_router::prelude::*;
use beet_tool::prelude::*;



pub fn stack_router() -> impl Bundle {
	(
		default_router(),
		OnSpawn::insert_child((
			route_tool("validate", Validate),
			ToolDescription::of::<Validate>(),
		)),
		OnSpawn::insert_child((
			route_tool("plan", Plan),
			ToolDescription::of::<Plan>(),
		)),
	)
}

/// Validate the stack
#[tool]
#[derive(Component, Reflect)]
#[reflect(Component)]
async fn Validate(cx: AsyncToolIn) -> Result<String> {
	cx.caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.build_project(entity)
		})
		.await?
		.validate()
		.await
}
/// Plan the stack
#[tool]
#[derive(Component, Reflect)]
#[reflect(Component)]
async fn Plan(cx: AsyncToolIn) -> Result<String> {
	cx.caller
		.with_state::<StackQuery, _>(|entity, query| {
			query.build_project(entity)
		})
		.await?
		.plan()
		.await
}



// fn build_config(mut commands: Commands) {
// 	let mut foo = 32;
// 	commands
// 		.entity(Entity::PLACEHOLDER)
// 		.call::<_, ()>(&mut foo, default());
// }
