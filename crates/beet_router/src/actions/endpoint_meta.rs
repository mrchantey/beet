use beet_core::prelude::*;
use beet_net::prelude::*;

/// Metadata for an endpoint
#[derive(Debug, Clone, Component)]
pub struct EndpointMeta {
	/// A collection of the content of every [`PathFilter`] in this entity's
	/// ancestors(inclusive)
	route_segments: RouteSegments,
}

impl EndpointMeta {
	/// Call [`RouteSegments::collect`] on this entity, collecting
	/// every parent [`PathFilter`]
	pub fn new(route_segments: RouteSegments) -> Self {
		Self { route_segments }
	}


	pub fn route_segments(&self) -> &RouteSegments { &self.route_segments }


	pub fn collect_all(
		query: Query<(Entity, &EndpointMeta)>,
	) -> Vec<(Entity, EndpointMeta)> {
		query
			.iter()
			.map(|(entity, segments)| (entity, segments.clone()))
			.collect::<Vec<_>>()
	}
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
			Endpoint,
			children![
				children![
					(
						PathFilter::new("*bar"),
						Endpoint
					),
					PathFilter::new("bazz")
				],
				(
					PathFilter::new("qux"),
				),
				(
					PathFilter::new(":quax"),
					Endpoint
				),
			],
		));
		world.run_system_cached(EndpointMeta::collect_all).unwrap()
    .into_iter()
    .map(|(_, meta)| meta.route_segments().annotated_route_path())
    .collect::<Vec<_>>()
		.xpect_eq(vec![
				RoutePath::new("/foo"),
				RoutePath::new("/foo/*bar"),
				RoutePath::new("/foo/:quax")
		]);
	}
}
