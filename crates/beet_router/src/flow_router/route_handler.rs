use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;


pub fn route_handler(path: PathFilter, method: HttpMethod) -> impl Bundle {
	(OnSpawn::new(collect_route_segments), Sequence, children![])
}

fn collect_route_segments(entity: &mut EntityWorldMut) {
	let id = entity.id();
	entity.world_scope(move |world| {
		let segments = world
			.run_system_cached_with(RouteSegments::collect, id)
			.unwrap();
		world.entity_mut(id).insert(segments);
	});
}


pub fn path_filter_check() -> impl Bundle {
	// OnSpawn::observe(||)
}
