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

		// #[cfg(all(
		// 	not(target_arch = "wasm32"),
		// 	not(test),
		// 	feature = "server"
		// ))]

		#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
		app.init_plugin_with(
			ServerPlugin::default()
				// user inserts their own server
				.without_server(),
		);
	}
}

/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	match EndpointTree::from_world(world) {
		Ok(tree) => {
			world.insert_resource(tree);
		}
		Err(err) => {
			error!("Failed to build EndpointTree: {}", err);
		}
	}
}


#[extend::ext(name=AsyncWorldRouterExt)]
pub impl AsyncWorld {
	/// Handle a single request and return the response
	/// ## Panics
	/// Panics if there is not exactly one `Router` in the world.
	fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> impl Future<Output = Result<Response>> {
		async move {
			let server = self
				.with_then(|world| {
					world.query_filtered::<Entity, With<Router>>().single(world)
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
	/// Panics if there is not exactly one `Router` in the world.
	fn oneshot(
		&mut self,
		req: impl Into<Request>,
	) -> impl Future<Output = Response> {
		let req = req.into();
		let entity = self
			.query_filtered::<Entity, With<Router>>()
			.single(self)
			.expect("Expected a single Router");
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

/// Added by default to the `required` Server for a [`Router`]
/// but can also be configured manually.
/// This handler differs from the default route handler in that
/// we use `beet_flow` primitives of GetOutcome / Outcome instead of
/// `Insert, Request`.
///
/// ## Example
///
/// ```rust,no_run
/// # use beet_router::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// World::new().spawn(HttpServer::default().with_handler(flow_route_handler));
/// ```
pub async fn flow_route_handler(
	router: AsyncEntity,
	request: impl Bundle,
) -> Response {
	let router_id = router.id();
	let (send, recv) = async_channel::bounded(1);
	router
		.world()
		.with_then(move |world| {
			let exchange = world
				.spawn((
					ExchangeOf(router_id),
					request,
					ExchangeContext::new(send),
				))
				.id();
			world
				.entity_mut(router_id)
				.trigger_target(GetOutcome.with_agent(exchange));
		})
		.await;
	recv.recv().await.unwrap_or_else(|_| {
		error!("Sender was dropped, was the world dropped?");
		Response::from_status(StatusCode::INTERNAL_SERVER_ERROR)
	})
}

// TODO rename to Router
/// The root of a server. In non-wasm non-lambda environments
/// this will also connect to a hyper server and listen for requests.
#[derive(Debug, Default, Clone, Component)]
#[require(ServerStatus)]
#[component(on_add=on_add)]
pub struct Router;

// On<Outcome> we need to pass the `exchange` [`Response`] to the
// [`ExchangeContext`], or else send a [`Response::not_found()`]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(
		move |ev: On<Outcome>, mut commands: Commands, route: RouteQuery| {
			let exchange = ev.agent();
			let path = route
				.path(&ev)
				.map(|p| p.xfmt_debug())
				.unwrap_or_else(|_| "unknown".to_string());
			// this observer
			commands.queue(move |world: &mut World| -> Result {
				let res = world
					.entity_mut(exchange)
					.take::<Response>()
					.unwrap_or_else(|| {
						Response::from_status_body(
							StatusCode::NOT_FOUND,
							format!("Resource not found at {path}"),
							"text/plain",
						)
					});
				let Some(cx) =
					world.entity_mut(exchange).take::<ExchangeContext>()
				else {
					bevybail!("Expected ExchangeContext on exchange entity. was an Outcome triggered without the agent attached?");
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


/// A [`Router`] that will map each [`Request`] and [`Response`] to a default [`HttpServer`]
#[cfg(feature = "server")]
#[derive(Debug, Default, Clone, Component)]
#[require(Router, HttpServer = HttpServer::default().with_handler(flow_route_handler))]
pub struct HttpRouter;

#[cfg(feature = "server")]
impl HttpRouter {
	/// Create a new `HttpRouter` bundle, using the test HttpServer in test environments
	pub fn new() -> impl Bundle + Clone {
		#[cfg(not(test))]
		{
			Self
		}
		#[cfg(test)]
		(
			HttpServer::new_test().with_handler(flow_route_handler),
			Self,
		)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[sweet::test]
	async fn works() {
		let mut world = RouterPlugin::world();
		world
			.spawn((Router, EndWith(Outcome::Pass)))
			.oneshot(Request::get("/foo"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
		// exchange entity was cleaned up
		world.query_once::<&ExchangeContext>().len().xpect_eq(0);
	}

	#[test]
	fn route_tree() {
		let mut world = World::new();
		world.spawn((Router, CacheStrategy::Static, children![
			EndpointBuilder::get()
				.with_handler(|tree: Res<EndpointTree>| tree.to_string()),
			(EndpointBuilder::get()
				.with_path("foo")
				.with_cache_strategy(CacheStrategy::Static)
				.with_handler(|| "foo")),
			(PathPartial::new("bar"), children![
				EndpointBuilder::get()
					.with_path("bazz")
					.with_cache_strategy(CacheStrategy::Static)
					.with_handler(|| "bazz")
			]),
			PathPartial::new("boo"),
		]));
		world.run_system_cached(insert_route_tree).unwrap();
		world
			.remove_resource::<EndpointTree>()
			.unwrap()
			.flatten()
			.iter()
			.map(|p| p.annotated_route_path())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				RoutePath::new("/"),
				RoutePath::new("/bar/bazz"),
				RoutePath::new("/foo"),
			]);
	}

	#[cfg(all(not(target_arch = "wasm32"), feature = "server"))]
	#[sweet::test(tokio)]
	async fn server() {
		let server = HttpServer::new_test().with_handler(flow_route_handler);
		let url = server.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, RouterPlugin))
				.spawn((server, Router, EndpointBuilder::get()))
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
