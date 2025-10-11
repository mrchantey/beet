use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;



pub struct FlowRouterPlugin;

impl Plugin for FlowRouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<BeetFlowPlugin>();

		#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
		app.init_plugin::<ServerPlugin>();
	}
}


#[extend::ext(name=WorldRouterExt)]
pub impl World {
	/// Handle a single request and return the response
	/// ## Panics
	/// Panics if there is not exactly one `RouteServer` in the world.
	fn oneshot(&mut self, req: Request) -> impl Future<Output = Response> {
		let entity = self
			.query_filtered::<Entity, With<RouteServer>>()
			.single(self)
			.expect("Expected a single RouteServer");
		self.run_async_then(async move |world| {
			route_handler(world.entity(entity), req)
				.await
				.into_response()
		})
	}
}
#[extend::ext(name=EntityWorldMutRouterExt)]
pub impl EntityWorldMut<'_> {
	/// Handle a single request and return the response
	fn oneshot(&mut self, req: Request) -> impl Future<Output = Response> {
		let entity = self.id();
		self.run_async_then(async move |world| {
			route_handler(world.entity(entity), req)
				.await
				.into_response()
		})
	}
}

async fn route_handler(
	entity: AsyncEntity,
	request: Request,
) -> Result<Response> {
	let world = entity.world();
	let exchange = world
		.spawn_then((request, RouteContextMap::default()))
		.await
		.id();
	let (send, recv) = async_channel::bounded(1);
	let _ = entity
		.observe(move |ev: On<Outcome>, mut commands: Commands| {
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
		.await
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

#[derive(Component)]
#[cfg_attr(all(not(target_arch = "wasm32"), feature = "server"),
	require(Server = Server::default().with_handler(route_handler))
)]
pub struct RouteServer;


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut world = (MinimalPlugins, FlowRouterPlugin).into_world();
		world.spawn((RouteServer, EndWith(Outcome::Pass)));
		world.all_entities().len().xpect_eq(1);
		world
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// agent was cleaned up
		world.all_entities().len().xpect_eq(1);
	}
}
