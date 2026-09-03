//! Shared fixtures for the world-bridge tests.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::resource::IS_RESOURCE;

/// A world with the async bridge installed and a type registry holding
/// [`Name`], the one component every bridge test writes through.
///
/// Async because every operation is served through a [`WorldBridge`], which
/// resolves at a sync point.
pub(crate) fn test_world() -> World {
	let world = AsyncPlugin::world();
	world
		.resource::<AppTypeRegistry>()
		.write()
		.register::<Name>();
	world
}

/// Serve one call line against `world`, the shape every bridged operation
/// takes.
///
/// Spawned as a task rather than polled inline: the bridge resolves at a sync
/// point, which drives the executor's tasks, not this one.
pub(crate) async fn serve(world: &mut World, call: &str) -> WorldReply {
	serve_with(world, call, ScriptConfig::default()).await
}

/// [`serve`], under an config narrower than the open default.
pub(crate) async fn serve_with(
	world: &mut World,
	call: &str,
	config: ScriptConfig,
) -> WorldReply {
	let call = serde_json::from_str::<WorldCall>(call).unwrap();
	world
		.run_async_then(move |world| async move {
			WorldBridge::new(world, config).serve(call).await
		})
		.await
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
