use crate::prelude::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use std::ops::ControlFlow;
/// Collection of systems for collecting and running and route handlers
pub struct Router;



impl Router {
	pub async fn oneshot_str(
		world: &mut World,
		req: impl Into<Request>,
	) -> Result<String> {
		let res = Self::oneshot(world, req).await.into_result().await?;
		res.text().await
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

	/// Handle a request in the world, returning the response
	pub async fn handle_request(
		mut world: World,
		request: Request,
	) -> (World, Response) {
		let start_time = CrossInstant::now();

		let route_parts = RouteParts::from_parts(&request.parts);

		trace!("Handling request: {:#?}", request);
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

		trace!("Returning Response: {:#?}", response);
		trace!("Route handler completed in: {:.2?}", start_time.elapsed());
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

		if let Some(handler) = world.entity(entity).get::<RouteHandler>() {
			world = handler.clone().run(world).await;
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
		world.spawn(RouteHandler::new(HttpMethod::Get, || "hello world!"));

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
			RouteHandler::new(HttpMethod::Get, || "hawaiian"),
		));
		Router::oneshot_str(&mut world, "sdjhkfds")
			.await
			.unwrap_err()
			.to_string()
			.xpect()
			.to_be("404 Not Found\n");
		Router::oneshot_str(&mut world, "/pizza")
			.await
			.unwrap()
			.xpect()
			.to_be_str("hawaiian");
	}
	#[sweet::test]
	async fn endpoint_with_children() {
		let mut world = World::new();
		world.spawn((
			RouteFilter::new("foo"),
			RouteHandler::new(HttpMethod::Get, || "foo"),
			children![(
				RouteFilter::new("bar"),
				RouteHandler::new(HttpMethod::Get, || "bar")
			),],
		));
		Router::oneshot_str(&mut world, "/foo")
			.await
			.unwrap()
			.xpect()
			.to_be_str("foo");
		Router::oneshot_str(&mut world, "/foo/bar")
			.await
			.unwrap()
			.xpect()
			.to_be_str("bar");
	}
}
