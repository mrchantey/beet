use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::ops::ControlFlow;
/// Collection of systems for collecting and running and route handlers
/// This type serves as the intermediarybetween the main app and the route handlers.
#[derive(Clone, Deref, DerefMut, Resource)]
pub struct Router {
	app_pool: AppPool,
}

impl Router {
	/// Create a new [`Router`] with the given plugin, which should add
	/// routes to the app either directly or in a [`Startup`] system.
	pub fn new(plugin: impl 'static + Send + Sync + Clone + Plugin) -> Self {
		Self {
			app_pool: AppPool::new(move || {
				let mut app = App::new();
				app.add_plugins((
					RouterPlugin,
					#[cfg(not(test))]
					LoadSnippetsPlugin,	
					plugin.clone()
				));
				app.init();
				app.update();
				app
			}),
		}
	}

	/// Handle a single request, returning the response or a 404 if not found.
	pub async fn oneshot(&self, req: impl Into<Request>) -> Response {
		self.handle_request(req.into()).await
	}
	pub async fn oneshot_str(&self, req: impl Into<Request>) -> Result<String> {
		let res = self.oneshot(req).await.into_result().await?;
		res.text().await
	}


	/// Handle a request in the world, returning the response
	pub async fn handle_request(&self, request: Request) -> Response {
		// let mut world = self.app_pool.pop();
		// TODO proper pooling, this creates new app each time
		let mut world = self.app_pool.pop();

		let start_time = CrossInstant::now();

		let route_parts = RouteParts::from_parts(&request.parts);

		trace!("Handling request: {:#?}", request);
		world.insert_resource(request);

		let roots = world.query_filtered_once::<Entity, Without<ChildOf>>();
		// let mut owned_world = std::mem::take(world);
		for entity in roots {
			world = self
				.handle_request_recursive(world, route_parts.clone(), entity)
				.await;
		}

		let response =
			if let Some(response) = world.remove_resource::<Response>() {
				response
			} else {
				// if no response try building one from a bundle
				bundle_to_html(world.inner_mut()).into_response()
			};

		trace!("Returning Response: {:#?}", response);
		trace!("Route handler completed in: {:.2?}", start_time.elapsed());

		response
	}
	/// Pre-order depth fist traversal. parent first, then children.
	async fn handle_request_recursive(
		&self,
		mut world: PooledWorld,
		parts: RouteParts,
		root_entity: Entity,
	) -> PooledWorld {
		struct StackFrame {
			entity: Entity,
			parts: RouteParts,
		}

		let mut stack = vec![StackFrame {
			entity: root_entity,
			parts,
		}];

		while let Some(StackFrame { entity, mut parts }) = stack.pop() {
			// Check 1: MethodFilter
			if let Some(method_filter) =
				world.entity(entity).get::<MethodFilter>()
			{
				if !method_filter.matches(&parts) {
					// method does not match, skip this entity
					continue;
				}
			}
			// Check 2: PathFilter
			if let Some(filter) = world.entity(entity).get::<PathFilter>() {
				match filter.matches(parts.clone()) {
					ControlFlow::Break(_) => {
						// path does not match, skip this entity
						continue;
					}
					ControlFlow::Continue(remaining_parts) => {
						parts = remaining_parts;
					}
				}
			}

			// at this point add children, even if the endpoint doesnt match
			// a child might
			if let Some(children) = world.entity(entity).get::<Children>() {
				// reverse children to maintain order with stack.pop()
				for child in children.iter().rev() {
					stack.push(StackFrame {
						entity: child,
						parts: parts.clone(),
					});
				}
			}

			// Check 3: Endpoint
			if let Some(endpoint) = world.entity(entity).get::<Endpoint>() {
				if
				// endpoints may only run if exact path match
				!parts.path().is_empty() || 
				// method must match
				endpoint.method() != parts.method()
				{
					continue;
				}
			}


			// Check 4: HandlerPredicates
			if let Some(predicates) =
				world.entity(entity).get::<HandlerConditions>().cloned()
			{
				let (world2, should_run) =
					predicates.should_run(std::mem::take(world.inner_mut()), entity).await;
				*world.inner_mut() = world2;
				if !should_run {
					continue;
				}
			}

			// Party time: actually run the handler
			if let Some(handler) =
				world.entity(entity).get::<RouteHandler>().cloned()
			{
				*world.inner_mut() = handler
					.clone()
					.run(std::mem::take(world.inner_mut()),entity)
					.await;
			}
		}
		world
	}
}




/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	let endpoints = world.run_system_cached(ResolvedEndpoint::collect).unwrap();
	let paths = endpoints
		.into_iter()
		.map(|(entity, endpoint)| (entity, endpoint.path().clone()))
		.collect();
	world.insert_resource(RoutePathTree::from_paths(paths));
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(Default, Resource, Deref, DerefMut)]
	struct Foo(Vec<u32>);

	#[sweet::test]
	async fn beet_route_works() {
		let router = Router::new(|app: &mut App| {
			app.world_mut()
				.spawn(RouteHandler::new(HttpMethod::Get, || "hello world!"));
		});
		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello world!");
	}


	async fn parse(route: &str) -> Vec<u32> {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				// PathFilter::new("/"),
				RouteHandler::layer(|mut res: ResMut<Foo>| {
					res.push(0);
				}),
			));
			app.world_mut().spawn(children![
				(
					PathFilter::new("foo"),
					RouteHandler::layer(|mut res: ResMut<Foo>| {
						res.push(1);
					}),
					children![
						(
							PathFilter::new("bar"),
							Endpoint::new(HttpMethod::Get),
							RouteHandler::layer(|mut res: ResMut<Foo>| {
								res.push(2);
							}),
						),
						(
							PathFilter::new("bazz"),
							Endpoint::new(HttpMethod::Delete),
							RouteHandler::layer(|mut res: ResMut<Foo>| {
								res.push(3);
							}),
						),
						(
							// no endpoint, always runs if parent matches
							RouteHandler::layer(|mut res: ResMut<Foo>| {
								res.push(4);
							}),
						),
					],
				),
				(RouteHandler::layer(
					|mut commands: Commands, res: ResMut<Foo>| {
						commands.insert_resource(
							Json(res.0.clone()).into_response(),
						);
					}
				),)
			]);

			app.world_mut().init_resource::<Foo>();
		});
		router
			.handle_request(Request::get(route))
			.await
			.json()
			.await
			.unwrap()
	}

	#[sweet::test]
	async fn works() {
		parse("/").await.xpect().to_be(vec![0]);
		parse("/foo").await.xpect().to_be(vec![0, 1, 4]);
		parse("/foo/chicken").await.xpect().to_be(vec![0, 1, 4]);
		parse("/foo/bar").await.xpect().to_be(vec![0, 1, 2, 4]);
		// path matches, method does not
		parse("/foo/bazz").await.xpect().to_be(vec![0, 1, 4]);
	}
	#[sweet::test]
	async fn simple() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				PathFilter::new("pizza"),
				RouteHandler::new(HttpMethod::Get, || "hawaiian"),
			));
		});
		router
			.oneshot_str("sdjhkfds")
			.await
			.unwrap_err()
			.to_string()
			.xpect()
			.to_be("404 Not Found\n");
		router
			.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hawaiian");
	}
	#[sweet::test]
	async fn endpoint_with_children() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				PathFilter::new("foo"),
				RouteHandler::new(HttpMethod::Get, || "foo"),
				children![(
					PathFilter::new("bar"),
					RouteHandler::new(HttpMethod::Get, || "bar")
				),],
			));
		});
		router
			.oneshot_str("/foo")
			.await
			.unwrap()
			.xpect()
			.to_be_str("foo");
		router
			.oneshot_str("/foo/bar")
			.await
			.unwrap()
			.xpect()
			.to_be_str("bar");
	}
	#[sweet::test]
	async fn route_tree() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				RouteHandler::new(
					HttpMethod::Get,
					|tree: Res<RoutePathTree>| tree.to_string(),
				),
				children![
					(
						PathFilter::new("foo"),
						RouteHandler::new(HttpMethod::Get, || "foo")
					),
					(PathFilter::new("bar"), children![(
						PathFilter::new("baz"),
						RouteHandler::new(HttpMethod::Get, || "baz")
					)]),
					(PathFilter::new("boo"),),
				],
			));
			app.world_mut()
				.run_system_cached(insert_route_tree)
				.unwrap();
		});
		router
			.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be_snapshot();
	}
}
