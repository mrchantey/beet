use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
/// Collection of systems for collecting and running and route handlers
pub struct Router;


fn static_routes(
	methods: Query<(Entity, &HttpMethod), With<StaticRoute>>,
	parents: Query<&ChildOf>,
	segments: Query<&RouteSegment>,
) -> Vec<RouteInfo> {
	let mut paths = Vec::new();
	for (entity, method) in methods.iter() {
		let mut path = Vec::new();
		for parent in parents.iter_ancestors_inclusive(entity) {
			match segments.get(parent) {
				Ok(RouteSegment::Static(route)) => {
					path.push(route.clone());
				}
				Ok(_) => {
					// TODO allow provided static segments
					// todo!("dynamic segments not collected");
					continue;
				}
				Err(_) => {
					// no segment, skip
				}
			}
		}
		path.reverse();
		paths.push(RouteInfo {
			method: method.clone(),
			path: RoutePath::new(path.join("/")),
		});
	}
	paths
}


impl Router {
	pub fn endpoints(world: &mut World) -> Vec<RouteInfo> {
		world.run_system_cached(static_routes).unwrap_or_default()
	}

	pub async fn oneshot(
		world: &mut World,
		req: impl Into<Request>,
	) -> Response {
		let world_owned = std::mem::take(world);
		let (world2, out) = Self::handle_request(world_owned, req.into()).await;
		*world = world2;
		out
	}

	/// For testing, collect all routes and return the base route as a string
	pub async fn oneshot_str(
		world: &mut World,
		route: impl Into<Request>,
	) -> Result<String> {
		Self::oneshot(world, route).await.xmap(|res| res.body_str())
	}


	/// Handle a request in the world, returning the response
	pub async fn handle_request(
		mut world: World,
		request: Request,
	) -> (World, Response) {
		let start_time = CrossInstant::now();

		let mut segments = request
			.parts
			.uri
			.path()
			.trim_start_matches('/')
			.split('/')
			.map(|s| s.to_string())
			.collect::<Vec<_>>();
		segments.reverse();
		let method = request.method();
		world.insert_resource(request);

		for entity in world.query_filtered_once::<Entity, Without<ChildOf>>() {
			world = handle_request_recursive(
				world,
				method,
				segments.clone(),
				entity,
			)
			.await;
		}

		if let Some(response) = world.remove_resource::<Response>() {
			return (world, response);
		}

		let response = world
			.remove_resource::<Response>()
			.unwrap_or_else(|| Response::not_found());

		debug!("Route handler completed in: {:.2?}", start_time.elapsed());

		(world, response)
	}
}



/// Pre-order dfs, accepting *reversed* path segments.
async fn handle_request_recursive(
	mut world: World,
	method: HttpMethod,
	// reversed path segments for pop()
	rev_segments: Vec<String>,
	root_entity: Entity,
) -> World {
	struct StackFrame {
		entity: Entity,
		rev_segments: Vec<String>,
	}

	let mut stack = vec![StackFrame {
		entity: root_entity,
		rev_segments,
	}];

	while let Some(StackFrame {
		entity,
		mut rev_segments,
	}) = stack.pop()
	{
		if let Some(entity_method) = world.entity(entity).get::<HttpMethod>() {
			if *entity_method != method {
				// method does not match, skip this entity
				continue;
			}
		}

		let current = rev_segments.pop();

		if let Some(segment) = world.entity(entity).get::<RouteSegment>() {
			match current {
				Some(current) if segment.matches(&current) => {}
				_ => {
					// segment does not match, dont execute handler or visit children
					continue;
				}
			}
		}


		if let Some(handler) = world.entity(entity).get::<RouteHandler>() {
			world = handler.clone().run(world).await;
		}
		if let Some(children) = world.entity(entity).get::<Children>() {
			// reverse children to maintain order with stack.pop()
			for child in children.iter().rev() {
				stack.push(StackFrame {
					entity: child,
					rev_segments: rev_segments.clone(),
				});
			}
		}
	}

	world
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
		let mut world = World::new();
		world.spawn(RouteHandler::new(|| "hello world!"));

		Router::oneshot_str(&mut world, "/")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hello world!");
	}


	async fn parse(route: &str) -> Vec<u32> {
		let mut world = World::new();


		// handlers without segments always run if root or their parent matches
		world.spawn(RouteHandler::new_layer(|mut res: ResMut<Foo>| {
			res.push(0);
		}));

		world.spawn((
			RouteSegment::new("foo"),
			RouteHandler::new_layer(|mut res: ResMut<Foo>| {
				res.push(1);
			}),
			children![
				(
					RouteSegment::new("bar"),
					RouteHandler::new_layer(|mut res: ResMut<Foo>| {
						res.push(2);
					}),
				),
				(
					StaticRoute,
					HttpMethod::Delete,
					RouteSegment::new("bazz"),
					RouteHandler::new_layer(|mut res: ResMut<Foo>| {
						res.push(3);
					}),
				),
				(
					// no segment, always runs if parent matches
					RouteHandler::new_layer(|mut res: ResMut<Foo>| {
						res.push(4);
					}),
				),
			],
		));


		world.init_resource::<Foo>();
		let (mut world, _response) =
			Router::handle_request(world, Request::get(route)).await;
		world.remove_resource::<Foo>().unwrap().0
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


	#[test]
	fn static_routes() {
		let mut world = World::new();
		world.spawn((
			StaticRoute,
			HttpMethod::Get,
			RouteSegment::new("foo"),
			children![
				children![(
					StaticRoute,
					HttpMethod::Post,
					RouteSegment::new("bar"),
				)],
				(StaticRoute, HttpMethod::Post, RouteSegment::new("bazz"),)
			],
		));
		Router::endpoints(&mut world).xpect().to_be(vec![
			RouteInfo::get("/foo"),
			RouteInfo::post("/foo/bar"),
			RouteInfo::post("/foo/bazz"),
		]);
	}
}
