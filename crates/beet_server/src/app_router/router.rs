use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::ops::ControlFlow;
/// Collection of systems for collecting and running and route handlers
pub struct Router;


fn static_routes(
	methods: Query<Entity, With<StaticRoute>>,
	parents: Query<&ChildOf>,
	filters: Query<&RouteFilter>,
) -> Vec<RouteInfo> {
	let mut paths = Vec::new();
	for entity in methods.iter() {
		let mut path = Vec::new();
		let mut method = HttpMethod::Get;
		// iterate over ancestors starting from the root
		for parent in parents
			.iter_ancestors_inclusive(entity)
			.collect::<Vec<_>>()
			.into_iter()
			.rev()
		{
			match filters.get(parent) {
				Ok(filter) => {
					for segment in filter.segments.iter() {
						match segment {
							RouteSegment::Static(s) => {
								path.push(s.to_string());
							}
							RouteSegment::Dynamic(str) => {
								path.push(format!("<dynamic-{}>", str));
							}
							RouteSegment::Wildcard(str) => {
								path.push(format!("<wildcard-{}>", str));
							}
						}
						if !filter.methods.is_empty() {
							method = filter.methods[0].clone();
						}
					}
				}
				Err(_) => {
					// no segment, skip
				}
			}
		}
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
		req: impl Into<Request>,
	) -> Result<String> {
		Self::oneshot(world, req).await.xmap(|res| res.text())
	}


	/// Handle a request in the world, returning the response
	pub async fn handle_request(
		mut world: World,
		request: Request,
	) -> (World, Response) {
		if request.parts.uri.path().starts_with("/.well-known/") {
			// skip 'well-known' requests
			return (world, Response::not_found());
		}

		let start_time = CrossInstant::now();

		let route_parts = RouteParts::from_parts(&request.parts);

		debug!("Handling request: {:#?}", request);
		world.insert_resource(request);

		for entity in world.query_filtered_once::<Entity, Without<ChildOf>>() {
			world =
				handle_request_recursive(world, route_parts.clone(), entity)
					.await;
		}

		let response =
			if let Some(response) = world.remove_resource::<Response>() {
				response
			} else {
				// if no response try building one from a bundle
				bundle_to_html(&mut world).into_response()
			};

		debug!("Returning Response: {:#?}", response);
		debug!("Route handler completed in: {:.2?}", start_time.elapsed());
		(world, response)
	}
}



/// Pre-order depth fist traversal. parent first, then children.
async fn handle_request_recursive(
	mut world: World,
	parts: RouteParts,
	root_entity: Entity,
) -> World {
	struct StackFrame {
		entity: Entity,
		parts: RouteParts,
	}

	let mut stack = vec![StackFrame {
		entity: root_entity,
		parts,
	}];

	while let Some(StackFrame { entity, mut parts }) = stack.pop() {

		if let Some(filter) = world.entity(entity).get::<RouteFilter>() {
			match filter.matches(parts.clone()) {
				ControlFlow::Break(_) => {
					// filter does not match, skip this entity
					continue;
				}
				ControlFlow::Continue(new_parts) => {
					parts = new_parts;
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
					parts: parts.clone(),
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
					RouteHandler::layer(|mut res: ResMut<Foo>| {
						res.push(2);
					}),
				),
				(
					StaticRoute,
					RouteFilter::new("bazz").with_method(HttpMethod::Delete),
					RouteHandler::layer(|mut res: ResMut<Foo>| {
						res.push(3);
					}),
				),
				(
					// no segment, always runs if parent matches
					RouteHandler::layer(|mut res: ResMut<Foo>| {
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
	#[sweet::test]
	async fn simple() {
		let mut world = World::new();
		world.spawn((
			RouteFilter::new("pizza"),
			RouteHandler::new(|| "hawaiian"),
		));
		Router::oneshot_str(&mut world, "sdjhkfds")
			.await
			.unwrap()
			.xpect()
			.to_be_str("Not Found");
		Router::oneshot_str(&mut world, "/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hawaiian");
	}




	#[test]
	fn static_routes() {
		let mut world = World::new();
		world.spawn((
			StaticRoute,
			RouteFilter::new("foo").with_method(HttpMethod::Get),
			children![
				children![(
					StaticRoute,
					RouteFilter::new("bar").with_method(HttpMethod::Post),
				),],
				(
					StaticRoute,
					RouteFilter::new("bazz").with_method(HttpMethod::Post),
				)
			],
		));
		Router::endpoints(&mut world).xpect().to_be(vec![
			RouteInfo::get("/foo"),
			RouteInfo::post("/foo/bar"),
			RouteInfo::post("/foo/bazz"),
		]);
	}
}
