use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


#[derive(Default)]
pub struct FlowRouterPlugin;

impl Plugin for FlowRouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<ControlFlowPlugin>();

		#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
		app.init_plugin_with(ServerPlugin::default().without_server());
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
			flow_route_handler(world.entity(entity), req)
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
			flow_route_handler(world.entity(entity), req)
				.await
				.into_response()
		})
	}
}
/// This handler differs from the default route handler in that
/// we use `beet_flow` primitives of GetOutcome / Outcome instead of
/// `Insert, Request`.
async fn flow_route_handler(
	server_async: AsyncEntity,
	request: Request,
) -> Response {
	let server = server_async.id();
	let (send, recv) = async_channel::bounded(1);
	server_async
		.world()
		.with_then(move |world| {
			let exchange = world
				.spawn((
					ExchangeOf(server),
					request,
					RouteContextMap::default(),
				))
				.id();
			world
				.entity_mut(server)
				.observe_any(move |ev: On<Outcome>, mut commands: Commands| {
					// this observer
					if ev.agent() == exchange {
						let send = send.clone();
						let observer = ev.observer();
						commands.queue(move |world: &mut World| {
							world.entity_mut(observer).despawn();
							let res = world
								.entity_mut(exchange)
								.take::<Response>()
								.unwrap_or_else(|| Response::not_found());
							world.entity_mut(exchange).despawn();
							send.try_send(res)
								.expect("unreachable, we await recv");
						});
					}
				})
				.trigger_target(GetOutcome.with_agent(exchange));
		})
		.await;

	recv.recv().await.unwrap_or_else(|_| {
		error!("Sender was dropped, was the world dropped?");
		Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	})
}

#[derive(Component)]
#[cfg_attr(all(not(target_arch = "wasm32"), feature = "server"),
	require(Server = Server::default().with_handler(flow_route_handler))
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
		let mut world = FlowRouterPlugin::world();
		world.all_entities().len().xpect_eq(0);
		world
			.spawn((RouteServer, EndWith(Outcome::Pass)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// agent was cleaned up
		world.all_entities().len().xpect_eq(1);
	}
}
