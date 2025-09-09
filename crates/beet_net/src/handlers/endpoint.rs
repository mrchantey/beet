use crate::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;


#[derive(SystemParam)]
pub struct EndpointQuery<'w, 's> {
	/// A query for route handlers
	query: Query<
		'w,
		's,
		(
			Entity,
			&'static RouteSegments,
			&'static RouteHandler,
			Option<&'static HttpMethod>,
			Option<&'static CacheStrategy>,
		),
		With<Endpoint>,
	>,
}
/// Collect all entities with a [`RouteHandler`]
pub fn endpoint_routes(
	endpoints: EndpointQuery,
) -> Vec<(Entity, RouteSegments)> {
	endpoints
		.query
		.iter()
		.map(|(entity, segments, _, _, _)| (entity, segments.clone()))
		.collect()
}
/// Collect all static GET endpoints from the world,
/// used for differentiating ssg paths:
/// - Method must be none or GET
/// - The cache strategy must be none or [`CacheStrategy::Static`]
/// - All segments must be [`PathSegment::Static`]
pub fn static_get_routes(endpoints: EndpointQuery) -> Vec<(Entity, RoutePath)> {
	endpoints
		.query
		.iter()
		.filter_map(|(entity, segments, _, method, strategy)| {
			if segments.is_static()
				&& (method == None || method == Some(&HttpMethod::Get))
				&& (strategy == None
					|| strategy == Some(&CacheStrategy::Static))
			{
				Some((entity, segments.annotated_route_path()))
			} else {
				None
			}
		})
		.collect()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;



	#[test]
	#[rustfmt::skip]
	fn collect() {
		let mut world = World::new();
		world.spawn((
			PathFilter::new("foo"),
			RouteHandler::ok(),
			children![
				children![
					(
						PathFilter::new("*bar"),
						HttpMethod::Post,
						RouteHandler::ok(),
					),
					PathFilter::new("bazz")
				],
				(
					PathFilter::new("qux"),
				),
				(
					PathFilter::new(":quax"),
					HttpMethod::Post,
					RouteHandler::ok(),
				),
			],
		));
		world.run_system_cached(endpoint_routes).unwrap()
    .into_iter()
    .map(|(_, segments)| segments.annotated_route_path())
    .collect::<Vec<_>>()
		.xpect_eq(vec![
				RoutePath::new("/foo"),
				RoutePath::new("/foo/*bar"),
				RoutePath::new("/foo/:quax")
		]);
	}
}
