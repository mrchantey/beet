use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;



pub struct FlowRouterPlugin;

impl Plugin for FlowRouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin(AsyncPlugin).init_plugin(BeetFlowPlugin);

		#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
		app.init_plugin(ServerPlugin)
			.init_resource::<ServerSettings>()
			.world_mut()
			.resource_mut::<ServerSettings>()
			.set_handler(route_handler);
	}
}


#[extend::ext(name=WorldRouterExt)]
pub impl World {
	/// Handle a single request and return the response, awaiting
	/// all async tasks to flush.
	fn oneshot(&mut self, req: Request) -> impl Future<Output = Response> {
		self.run_async_then(async move |world| {
			route_handler(world, req).await.into_response()
		})
	}
}

async fn route_handler(
	world: AsyncWorld,
	request: Request,
) -> Result<Response> {
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

	let exchange = world.spawn_then(request).await;
	let (send, recv) = async_channel::bounded(1);
	let _ = world
		.entity(root)
		.observe(move |ev: On<GetOutcome>, mut commands: Commands| {
			if ev.agent() == exchange {
				let send = send.clone();
				let observer = ev.observer();
				commands.queue(move |world: &mut World| {
					world.entity_mut(observer).despawn();
					let res = world
						.entity_mut(exchange)
						.take::<Response>()
						.unwrap_or_else(|| Response::not_found());
					let _ = send.try_send(res);
				});
			}
		})
		.await;

	world
		.entity(root)
		.trigger_target(GetOutcome.with_agent(exchange))
		.await;


	let res = recv.recv().await.map_err(|e| {
		HttpError::new(
			StatusCode::INTERNAL_SERVER_ERROR,
			format!("Failed to receive response: {e}"),
		)
	})?;
	world.entity(exchange).despawn().await;
	res.xok()
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, FlowRouterPlugin));
		let world = app.world_mut();
		// let mut world = (MinimalPlugins, FlowRouterPlugin).into_world();
		world.spawn((RouterRoot, EndWith(Outcome::Pass)));
		world.all_entities().len().xpect_eq(1);
		world
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		world.all_entities().len().xpect_eq(1);
	}
}
