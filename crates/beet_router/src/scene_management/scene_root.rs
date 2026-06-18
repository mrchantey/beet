//! Core scene-management primitives, shared by every host of a loadable beet
//! scene (the file-watching CLI, the HTTP scene server, …): the
//! [`BeetSceneRoot`] marker, the [`ResetScene`] event and [`set_scene`], which
//! swaps the active scene atomically.

use crate::prelude::*;
use beet_core::prelude::*;

extern crate alloc;
use alloc::vec::Vec;

/// Marks a root entity spawned from the active beet scene, so the whole scene can
/// be despawned wholesale on reload. Named to avoid clashing with bevy's own
/// `SceneRoot` (a handle to a scene asset).
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct BeetSceneRoot;

/// Triggered just before the active scene is despawned, so scene behaviours can
/// return to a resting state (eg stop a robot, turn off LEDs, close a connection)
/// before their entities are removed. An extension point: the host triggers it,
/// loaded scenes and hardware plugins add observers.
#[derive(Event)]
pub struct ResetScene;

/// Despawn the active scene (if any), then spawn the scene described by `media`,
/// marking each spawned root [`BeetSceneRoot`]. Returns the new roots.
///
/// Runs under one exclusive world lock so no frame observes a half-swapped scene:
/// the old [`BeetSceneRoot`] trees are despawned (after a [`ResetScene`] trigger,
/// so hardware returns to rest), the new scene is deserialized and its roots
/// marked. When `parent` is given the roots are reparented under it and the
/// [`RouteTree`] is rebuilt explicitly, since reparenting does not retrigger
/// route-tree construction; without a parent the roots stay where loaded and the
/// [`rebuild_route_trees_on_load`] observer recomputes the tree.
pub fn set_scene(
	world: &mut World,
	media: &MediaBytes,
	parent: Option<Entity>,
) -> Result<Vec<Entity>> {
	despawn_scene(world);

	// roots are the spawned entities with no parent; mark them so the whole
	// scene can be despawned together on the next swap.
	let roots = TemplateLoader::new(world)
		.load(media)?
		.into_iter()
		.filter(|entity| !world.entity(*entity).contains::<ChildOf>())
		.collect::<Vec<_>>();
	roots.iter().for_each(|root| {
		world.entity_mut(*root).insert(BeetSceneRoot);
	});

	if let Some(parent) = parent {
		roots.iter().for_each(|root| {
			world.entity_mut(*root).insert(ChildOf(parent));
		});
		// reparenting does not retrigger route-tree construction, so rebuild it
		// explicitly from the parent's (now larger) descendant set.
		world
			.run_system_cached_with(RouteTree::rebuild, parent)
			.ok()
			.transpose()?;
	}
	Ok(roots)
}

/// Despawn the active scene: trigger [`ResetScene`] then despawn every
/// [`BeetSceneRoot`] tree, rebuilding the route tree of each server the roots
/// hung under so the cleared routes drop out of dispatch. A no-op when no scene
/// is loaded.
pub fn despawn_scene(world: &mut World) {
	let existing = world
		.query_filtered::<Entity, With<BeetSceneRoot>>()
		.iter(world)
		.collect::<Vec<_>>();
	if existing.is_empty() {
		return;
	}
	world.trigger(ResetScene);
	// the servers the scene was reparented under, captured before despawning so
	// their route trees can be rebuilt without the now-gone routes.
	let servers = existing
		.iter()
		.filter_map(|entity| {
			world.entity(*entity).get::<ChildOf>().map(ChildOf::parent)
		})
		.collect::<HashSet<_>>();
	existing
		.into_iter()
		.for_each(|entity| world.entity_mut(entity).despawn());
	servers.into_iter().for_each(|server| {
		world
			.run_system_cached_with(RouteTree::rebuild, server)
			.unwrap_or(Ok(()))
			.ok();
	});
}


#[cfg(all(test, feature = "template_serde", feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[action(handler_only)]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Ping(_cx: ActionContext<RequestParts>) -> MediaBytes {
		MediaBytes::new_text("pong")
	}

	fn test_world() -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		world.resource::<AppTypeRegistry>().write().register::<Ping>();
		world
	}

	/// A serialized router scene reloads via [`set_scene`] with its routes
	/// reconstructed and reparented under a server, proving the reflectable
	/// markers rebuild their path/behaviour from their require hooks.
	#[beet_core::test(timeout_ms = 10000)]
	async fn set_scene_round_trips() {
		// build + serialize a one-route scene, as an exporter would.
		let mut world = test_world();
		let root = world
			.spawn((default_router(), children![exchange_route("ping", Ping)]))
			.flush();
		let json = TemplateSaver::new()
			.with_entity_tree(&world, root)
			.save(&world, MediaType::Json)
			.unwrap();

		// load it under a fresh server entity, as the scene server does.
		let mut world = test_world();
		let server = world.spawn(default_router()).flush();
		let roots = set_scene(&mut world, &json, Some(server)).unwrap();
		roots.len().xpect_eq(1);
		world.flush();

		let tree = world.entity(server).get::<RouteTree>().unwrap();
		tree.find(&["ping"]).xpect_some();
	}

	/// A pushed scene round-trips: load a scene under a host, mark it
	/// [`BeetSceneRoot`], then re-serialize via `save_roots_filtered` (what `dump`
	/// reads off a device) and reload it. The child routes must survive the
	/// round-trip, not just the bare root.
	#[beet_core::test(timeout_ms = 10000)]
	async fn scene_round_trip_keeps_children() {
		// build + serialize a one-route scene, as an exporter would.
		let mut world = test_world();
		let root = world
			.spawn((default_router(), children![exchange_route("ping", Ping)]))
			.flush();
		let json = TemplateSaver::new()
			.with_entity_tree(&world, root)
			.save(&world, MediaType::Json)
			.unwrap();

		// a device receives and re-dumps a scene repeatedly (push, dump, re-push),
		// so the cycle must survive more than once. Each iteration loads the prior
		// bytes under a fresh host and re-saves the `BeetSceneRoot` trees; the child
		// route must never be dropped.
		let mut bytes = json;
		for _ in 0..3 {
			let mut world = test_world();
			let host = world.spawn(default_router()).flush();
			set_scene(&mut world, &bytes, Some(host)).unwrap();
			world.flush();
			world
				.entity(host)
				.get::<RouteTree>()
				.unwrap()
				.find(&["ping"])
				.xpect_some();
			bytes = TemplateSaver::new()
				.save_roots_filtered::<With<BeetSceneRoot>>(&mut world, MediaType::Json)
				.unwrap();
		}
	}
}
