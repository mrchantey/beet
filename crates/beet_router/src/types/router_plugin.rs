use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;


#[derive(Default)]
pub struct RouterPlugin;

impl Plugin for RouterPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.init_plugin::<ApplyDirectivesPlugin>()
			.init_plugin::<ControlFlowPlugin>()
			.init_resource::<WorkspaceConfig>()
			.init_resource::<RenderMode>()
			.init_resource::<HtmlConstants>()
			.add_systems(PostStartup, insert_route_tree);

		#[cfg(not(test))]
		app.init_plugin::<LoadSnippetsPlugin>();

		#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
		app.init_plugin_with(
			ServerPlugin::default()
				// user inserts their own server
				.without_server(),
		);

		#[cfg(feature = "lambda")]
		app.add_plugins(lambda_plugin);
	}
}

/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	let paths = world
		.run_system_cached(EndpointMeta::collect.pipe(EndpointMeta::static_get))
		.unwrap()
		.into_iter()
		.map(|meta| {
			(meta.entity(), meta.route_segments().annotated_route_path())
		})
		.collect::<Vec<_>>();
	world.insert_resource(RoutePathTree::from_paths(paths));
}


#[extend::ext(name=AsyncWorldRouterExt)]
pub impl AsyncWorld {
	/// Handle a single request and return the response
	/// ## Panics
	/// Panics if there is not exactly one `RouteServer` in the world.
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Result<Response>> {
		async move {
			let server = self
				.with_then(|world| {
					world
						.query_filtered::<Entity, With<RouteServer>>()
						.single(world)
				})
				.await?;
			self.entity(server).oneshot(req).await
		}
	}
}
#[extend::ext(name=AsyncEntityRouterExt)]
pub impl AsyncEntity {
	/// Handle a single request and return the response
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Result<Response>> {
		async move { flow_route_handler(self.clone(), req.into()).await.xok() }
	}
}
#[extend::ext(name=WorldRouterExt)]
pub impl World {
	/// Handle a single request and return the response
	/// ## Panics
	/// Panics if there is not exactly one `RouteServer` in the world.
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let req = req.into();
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
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let req = req.into();
		self.oneshot_bundle(req)
	}
	/// Handle a single request bundle and return the response
	fn oneshot_bundle(
		&mut self,
		bundle: impl Bundle,
	) -> impl Future<Output = Response> {
		self.run_async_then(async move |entity| {
			flow_route_handler(entity, bundle).await.into_response()
		})
	}

	#[cfg(test)]
	/// Convenience method for testing, unwraps a 200 response string
	fn oneshot_str(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = String> {
		let req = req.into();
		async {
			self.oneshot(req)
				.await
				.into_result()
				.await
				.unwrap()
				.text()
				.await
				.expect("Expected text body")
		}
	}
}

/// Added by default to the `required` Server for a [`RouteServer`]
/// but can also be configured manually.
/// This handler differs from the default route handler in that
/// we use `beet_flow` primitives of GetOutcome / Outcome instead of
/// `Insert, Request`.
///
/// ## Example
///
/// ```rust
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// World::new().spawn(Server::default().with_handler(flow_route_handler));
/// ```
pub async fn flow_route_handler(
	server_async: AsyncEntity,
	request: impl Bundle,
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
					ExchangeContext::new(send),
				))
				.id();
			world
				.entity_mut(server)
				.trigger_target(GetOutcome.with_agent(exchange));
		})
		.await;

	recv.recv().await.unwrap_or_else(|_| {
		error!("Sender was dropped, was the world dropped?");
		Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	})
}


/// The root of a server. In non-wasm non-lambda environments
/// this will also connect to a hyper server and listen for requests.
#[derive(Clone, Component)]
#[require(ServerStatus)]
#[cfg_attr(all(not(target_arch = "wasm32"), not(feature = "lambda"), feature = "server"),
	require(Server = Server::default().with_handler(flow_route_handler))
)]
#[component(on_add=on_add)]
pub struct RouteServer;

// On<Outcome> we need to pass the `exchange` [`Response`] to the
// [`ExchangeContext`], or else send a [`Response::not_found()`]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(
		move |ev: On<Outcome>, mut commands: Commands| {
			let exchange = ev.agent();
			// this observer
			commands.queue(move |world: &mut World| -> Result {
				let res = world
					.entity_mut(exchange)
					.take::<Response>()
					.unwrap_or_else(|| Response::not_found());
				let Some(cx) =
					world.entity_mut(exchange).take::<ExchangeContext>()
				else {
					bevybail!("Expected ExchangeContext on exchange entity");
				};
				world.entity_mut(exchange).despawn();
				cx.sender().try_send(res).map_err(|_| {
					bevyhow!("Failed to send, was the receiver dropped?")
				})?;
				Ok(())
			});
		},
	);
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
		let mut world = RouterPlugin::world();
		world.all_entities().len().xpect_eq(0);
		world
			.spawn((RouteServer, EndWith(Outcome::Pass)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// exchange entity was cleaned up
		world.all_entities().len().xpect_eq(1);
	}

	#[sweet::test]
	async fn route_tree() {
		let mut world = World::new();
		world.spawn((RouteServer, CacheStrategy::Static, children![
			EndpointBuilder::get()
				.with_handler(|tree: Res<RoutePathTree>| tree.to_string()),
			(
				PathFilter::new("foo"),
				EndpointBuilder::get()
					.with_path("foo")
					.with_cache_strategy(CacheStrategy::Static)
					.with_handler(|| "foo")
			),
			(PathFilter::new("bar"), children![
				EndpointBuilder::get()
					.with_path("bazz")
					.with_cache_strategy(CacheStrategy::Static)
					.with_handler(|| "bazz")
			]),
			PathFilter::new("boo"),
		]));
		world.run_system_cached(insert_route_tree).unwrap();
		world
			.remove_resource::<RoutePathTree>()
			.unwrap()
			.flatten()
			.xpect_eq(vec![
				RoutePath::new("/bar/bazz"),
				RoutePath::new("/foo"),
			]);
	}

	#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
	#[sweet::test]
	async fn server() {
		let server = Server::new_test().with_handler(flow_route_handler);
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((server, RouteServer, EndpointBuilder::get()))
				.run();
		});
		time_ext::sleep_millis(10).await;
		Request::get(&url)
			.send()
			.await
			.unwrap()
			.status()
			.xpect_eq(StatusCode::OK);
	}
}
