//! Shared fixtures for the world-bridge tests.
use beet_core::prelude::*;
use bevy::ecs::resource::IS_RESOURCE;

/// A world with a type registry holding [`Name`], the one component every
/// bridge test writes through.
pub(crate) fn test_world() -> World {
	let mut world = World::new();
	world.init_resource::<AppTypeRegistry>();
	world
		.resource::<AppTypeRegistry>()
		.write()
		.register::<Name>();
	world
}

/// The entities a scene would consider its own.
///
/// A resource is an entity too, so a bare `iter_entities` count answers a
/// different question than the one these tests are asking.
pub(crate) fn entity_count(world: &World) -> usize {
	world
		.iter_entities()
		.filter(|entity| !entity.contains_id(IS_RESOURCE))
		.count()
}

/// The one [`Name`] in the world, for the tests that write exactly one.
pub(crate) fn only_name(world: &mut World) -> Name {
	world.query::<&Name>().iter(world).next().cloned().unwrap()
}
