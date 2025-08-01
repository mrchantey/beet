use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::ops::ControlFlow;
/// Collection of systems for collecting and running and route handlers
/// This type serves as the intermediarybetween the main app and the route handlers.
#[derive(Clone, Deref, DerefMut,Resource)]
pub struct Router{
	app_pool:AppPool
}

impl Router {
/// Create a new [`Router`] with the given plugin, which should add
/// routes to the app either directly or in a [`Startup`] system.
	pub fn new(plugin:impl 'static + Send + Sync + Clone + Plugin)->Self{
		Self{
			app_pool:AppPool::new(move || {
				let mut app = App::new();
				app.add_plugins((RouterPlugin,plugin.clone()));
				app.init();
				app.update();
				app
			}) 
	}
}

	/// Handle a single request, returning the response or a 404 if not found.
	pub async fn oneshot(
		&self,
		req: impl Into<Request>,
	) -> Response {
		self.handle_request(req.into()).await
	}
		pub async fn oneshot_str(
			&self,
		req: impl Into<Request>,
	) -> Result<String> {
		let res = self.oneshot(req).await.into_result().await?;
		res.text().await
	}


	/// Handle a request in the world, returning the response
	pub async fn handle_request(
		&self,
		request: Request,
	) -> Response {

		let mut app = self.app_pool.get();
		let world = app.world_mut();

		let start_time = CrossInstant::now();

		let route_parts = RouteParts::from_parts(&request.parts);

		trace!("Handling request: {:#?}", request);
		world.insert_resource(request);

		for entity in world.query_filtered_once::<Entity, Without<ChildOf>>() {
				self.handle_request_recursive(route_parts.clone(), entity)
					.await;
		}

		let response =
			if let Some(response) = world.remove_resource::<Response>() {
				response
			} else {
				// if no response try building one from a bundle
				bundle_to_html(world).into_response()
			};

		trace!("Returning Response: {:#?}", response);
		trace!("Route handler completed in: {:.2?}", start_time.elapsed());
		response
	}
	/// Pre-order depth fist traversal. parent first, then children.
	async fn handle_request_recursive(
		&self,
		parts: RouteParts,
		root_entity: Entity,
	) {
	let mut app = self.app_pool.get();
		let world = app.world_mut();
	
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
			if let Some(method_filter) = world.entity(entity).get::<MethodFilter>()
			{
				if !method_filter.matches(&parts) {
					// method does not match, skip this entity
					continue;
				}
			}
			// Check 2: RouteFilter
			if let Some(filter) = world.entity(entity).get::<RouteFilter>() {
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
				// endpoints may only run if exact match
				!parts.path().is_empty() || 
				// method must match
				endpoint.method() != parts.method() {
					continue;
				}
			}
	
			// Party time: actually run the handler
			if let Some(handler) = world.entity(entity).get::<RouteHandler>().cloned() {
						let world_owned = std::mem::take(world);
						let returned_world = handler.run(world_owned).await;
			}
		}
	}
}




/// insert a route tree for the current world, added at startup by the [`RouterPlugin`].
pub fn insert_route_tree(world: &mut World) {
	let endpoints = world.run_system_cached(ResolvedEndpoint::collect).unwrap();
	let paths = endpoints.into_iter().map(|(entity, endpoint)| (entity, endpoint.path().clone())).collect();
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
		app.world_mut().spawn(RouteHandler::new(HttpMethod::Get, || "hello world!"));
	});
	router.oneshot_str("/")
		.await
		.unwrap()
		.xpect()
		.to_be_str("hello world!");
	}


	async fn parse(route: &str) -> Vec<u32> {
		let mut world = World::new();


		world.spawn((
			// RouteFilter::new("/"),
			RouteHandler::layer(|mut res: ResMut<Foo>| {
				res.push(0);
			}),
		));

		world.spawn((
			RouteFilter::new("foo"),
			RouteHandler::layer(|mut res: ResMut<Foo>| {
				res.push(1);
			}),
			children![
				(
					RouteFilter::new("bar"),
					Endpoint::new(HttpMethod::Get),
					RouteHandler::layer(|mut res: ResMut<Foo>| {
						res.push(2);
					}),
				),
				(
					RouteFilter::new("bazz"),
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
		));


	let router = Router::new(|app: &mut App| {
		app.world_mut().spawn((
			// RouteFilter::new("/"),
			RouteHandler::layer(|mut res: ResMut<Foo>| {
				res.push(0);
			}),
		));
		app.world_mut().spawn((
			RouteFilter::new("foo"),
			RouteHandler::layer(|mut res: ResMut<Foo>| {
				res.push(1);
			}),
			children![
				(
					RouteFilter::new("bar"),
					Endpoint::new(HttpMethod::Get),
					RouteHandler::layer(|mut res: ResMut<Foo>| {
						res.push(2);
					}),
				),
				(
					RouteFilter::new("bazz"),
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
		));
		app.world_mut().init_resource::<Foo>();
	});
	router.handle_request(Request::get(route)).await;
	// Remove Foo resource from the world inside the router's app_pool
	let mut app = router.app_pool.get();
	app.world_mut().remove_resource::<Foo>().unwrap().0;
	todo!("pretty sure this wont work");
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
				RouteFilter::new("pizza"),
				RouteHandler::new(HttpMethod::Get, || "hawaiian"),
			));
		});
		router.oneshot_str("sdjhkfds")
			.await
			.unwrap_err()
			.to_string()
			.xpect()
			.to_be("404 Not Found\n");
		router.oneshot_str("/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hawaiian");
	}
	#[sweet::test]
	async fn endpoint_with_children() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				RouteFilter::new("foo"),
				RouteHandler::new(HttpMethod::Get, || "foo"),
				children![(
					RouteFilter::new("bar"),
					RouteHandler::new(HttpMethod::Get, || "bar")
				),],
			));
		});
		router.oneshot_str("/foo")
			.await
			.unwrap()
			.xpect()
			.to_be_str("foo");
		router.oneshot_str("/foo/bar")
			.await
			.unwrap()
			.xpect()
			.to_be_str("bar");
	}
	#[sweet::test]
	async fn route_tree() {
		let router = Router::new(|app: &mut App| {
			app.world_mut().spawn((
				RouteHandler::new(HttpMethod::Get, |tree: Res<RoutePathTree>| {
					tree.to_string()
				}),
				children![
					(
						RouteFilter::new("foo"),
						RouteHandler::new(HttpMethod::Get, || "foo")
					),
					(
						RouteFilter::new("bar"),
						children![
							(
								RouteFilter::new("baz"),
								RouteHandler::new(HttpMethod::Get, || "baz")
							)]
					),
					(
						RouteFilter::new("boo"),
					),
				]
			));
			app.world_mut().run_system_cached(insert_route_tree).unwrap();
		});
		router.oneshot_str("/")
			.await
			.unwrap()
			.xpect()
			.to_be_snapshot();

	}
}
