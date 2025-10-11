use beet_core::prelude::*;
use beet_net::prelude::*;


/// Call [`RouteSegments::collect`] on this entity, collecting
/// every parent [`PathFilter`]
pub fn collect_route_segments() -> impl Bundle {
	OnSpawn::new(|entity| {
		let id = entity.id();
		entity.world_scope(move |world| {
			let segments = world
				.run_system_cached_with(RouteSegments::collect, id)
				.unwrap();
			world.entity_mut(id).insert(segments);
		});
	})
}


pub fn collect_endpoint_routes(
	query: Query<(Entity, &RouteSegments)>,
) -> Vec<(Entity, RouteSegments)> {
	query
		.iter()
		.map(|(entity, segments)| (entity, segments.clone()))
		.collect::<Vec<_>>()
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;



	#[test]
	#[rustfmt::skip]
	fn test_collect_route_segments() {
		let mut world = World::new();
		world.spawn((
			PathFilter::new("foo"),
			collect_route_segments(),
			children![
				children![
					(
						PathFilter::new("*bar"),
						collect_route_segments()
					),
					PathFilter::new("bazz")
				],
				(
					PathFilter::new("qux"),
				),
				(
					PathFilter::new(":quax"),
					collect_route_segments()
				),
			],
		));
		world.run_system_cached(collect_endpoint_routes).unwrap()
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
