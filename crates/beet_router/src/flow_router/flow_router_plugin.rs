use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;



pub struct FlowRouterPlugin;

impl Plugin for FlowRouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(BeetFlowPlugin).init_plugin(ServerPlugin);
		app.world_mut()
			.resource_mut::<ServerSettings>()
			.set_handler(route_handler);
	}
}


async fn route_handler(world: AsyncWorld, request: Request) -> Response {
	let root = world
		.with_then(|world| {
			world
				.query_filtered::<Entity, With<RouterRoot>>()
				.single(world)
		})
		.await
		.map_err(|e| {
			HttpError::new(
				StatusCode::INTERNAL_SERVER_ERROR,
				format!("No RouterRoot found: {e}"),
			)
		})?;


	Response::ok()
}
