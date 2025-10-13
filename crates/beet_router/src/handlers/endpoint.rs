use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::SystemParam;


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
