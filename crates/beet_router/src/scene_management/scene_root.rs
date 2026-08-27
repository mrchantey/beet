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
pub(crate) fn set_scene(
	world: &mut World,
	media: &MediaBytes,
	parent: Option<Entity>,
) -> Result<Vec<Entity>> {
	BeetSceneRoot::despawn_all(world);

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

impl BeetSceneRoot {
	/// Despawn the active scene: trigger [`ResetScene`] then despawn every
	/// [`BeetSceneRoot`] tree and the [`SceneResource`]-backed resources *those roots*
	/// declared (so a rebuild reinserts each fresh from markup rather than patching
	/// stale live state), rebuilding the route tree of each server the roots hung
	/// under so the cleared routes drop out of dispatch. A no-op when no scene is
	/// loaded.
	///
	/// Resource removal is scoped by [`SceneResource::root`], never global: a
	/// non-scene build root (a `serve`/`export-static` entry, which carries no
	/// [`BeetSceneRoot`]) owns its resources for the process lifetime, and must not
	/// lose them when a scene swaps underneath it.
	pub fn despawn_all(world: &mut World) {
		let existing = world
			.query_filtered::<Entity, With<BeetSceneRoot>>()
			.iter(world)
			.collect::<HashSet<_>>();
		// the resources these roots created, plus any whose root is already gone: an
		// orphan can never be rebuilt from markup, and leaving it live would make the
		// next build patch a stale resource instead of creating a fresh one.
		let resources = world
			.query::<(Entity, &SceneResource)>()
			.iter(world)
			.filter(|(_, owned)| {
				existing.contains(&owned.root)
					|| world.get_entity(owned.root).is_err()
			})
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		if existing.is_empty() && resources.is_empty() {
			return;
		}
		world.trigger(ResetScene);
		// fully remove each markup-created resource: take the resource component off
		// its backing entity, discard the `IsResource` caretaker (whose hook cleans
		// bevy's resource cache; flushed so the queued cleanup lands on a live
		// entity), then despawn the husk. Bevy warns on despawning a *live* resource
		// entity, so the caretaker must go first.
		for entity in resources {
			use bevy::ecs::resource::IsResource;
			if let Some(id) = world
				.entity(entity)
				.get::<IsResource>()
				.map(IsResource::resource_component_id)
			{
				world.entity_mut(entity).remove_by_id(id);
			}
			world.entity_mut(entity).remove::<IsResource>();
			world.flush();
			world.entity_mut(entity).despawn();
		}
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
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Ping>();
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
			.spawn((Router::with_defaults(), children![route::exchange(
				"ping", Ping
			)]))
			.flush();
		let json = TemplateSaver::new()
			.with_entity_tree(&world, root)
			.save(&world, MediaType::Json)
			.unwrap();

		// load it under a fresh server entity, as the scene server does.
		let mut world = test_world();
		let server = world.spawn(Router::with_defaults()).flush();
		let roots = set_scene(&mut world, &json, Some(server)).unwrap();
		roots.len().xpect_eq(1);
		world.flush();

		// the scene is rooted in its own `Router`, so it mounts under the host
		// as a url space of its own and owns the tree its routes land in.
		RouteTree::of(&world, roots[0])
			.unwrap()
			.find(&["ping"])
			.xpect_some();
		// and it lands there ALONE: the phantom-tree class this guards against
		// left "ping" split across two `RouteTree`s (one on the pre-reparent
		// element, one on the real url space), dispatching from neither. Only
		// `set_scene`'s own explicit rebuild ran here, no manual poke beyond it.
		world
			.query::<(Entity, &RouteTree)>()
			.iter(&world)
			.filter(|(_, tree)| tree.find(&["ping"]).is_some())
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>()
			.xpect_eq(vec![roots[0]]);
	}

	/// A markup-declared resource is scene-owned: `despawn_scene` removes it
	/// with the scene, and a rebuild reinserts it fresh from markup (the create
	/// branch again, not a patch of stale live state).
	#[beet_core::test]
	async fn despawn_scene_removes_markup_resources() {
		// no pre-inserted `PackageConfig` (unlike `test_world`), so the markup
		// *creates* the resource and owns its lifetime.
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<PackageConfig>();
		let spawn = |world: &mut World| -> Entity {
			let nodes = BsxNode::parse_document(
				r#"<PackageConfig title="Owned"/>"#,
				&BsxParseConfig::bsx(),
			)
			.unwrap();
			let root = world
				.spawn_template(BsxTemplate::container(
					nodes,
					BsxTemplateRegistry::default(),
				))
				.unwrap()
				.id();
			world.entity_mut(root).insert(BeetSceneRoot);
			root
		};
		spawn(&mut world);
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("Owned");
		// teardown removes the markup-created resource with the scene
		BeetSceneRoot::despawn_all(&mut world);
		world.get_resource::<PackageConfig>().xpect_none();
		// a rebuild reinserts it fresh from markup
		spawn(&mut world);
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("Owned");
	}

	/// Resource teardown is scoped to the roots being despawned: a resource
	/// declared by a build root that is *not* a [`BeetSceneRoot`] (a `serve` /
	/// `export-static` entry) survives a scene swap. A global sweep would take it,
	/// leaving the host entry without the config it declared.
	#[beet_core::test]
	async fn despawn_scene_keeps_other_roots_resources() {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<PackageConfig>();
		// the host entry declares the resource and owns it, with no `BeetSceneRoot`
		let entry = world
			.spawn_template(BsxTemplate::container(
				BsxNode::parse_document(
					r#"<PackageConfig title="Host"/>"#,
					&BsxParseConfig::bsx(),
				)
				.unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("Host");

		// a scene loads and is torn down beside it
		let scene = world.spawn_template(()).unwrap().id();
		world.entity_mut(scene).insert(BeetSceneRoot);
		BeetSceneRoot::despawn_all(&mut world);

		// the scene is gone, the entry and its resource are untouched
		world.get_entity(scene).is_err().xpect_true();
		world.get_entity(entry).is_ok().xpect_true();
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("Host");
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
			.spawn((Router::with_defaults(), children![route::exchange(
				"ping", Ping
			)]))
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
			let host = world.spawn(Router::with_defaults()).flush();
			let roots = set_scene(&mut world, &bytes, Some(host)).unwrap();
			world.flush();
			RouteTree::of(&world, roots[0])
				.unwrap()
				.find(&["ping"])
				.xpect_some();
			bytes = TemplateSaver::new()
				.save_roots_filtered::<With<BeetSceneRoot>>(
					&mut world,
					MediaType::Json,
				)
				.unwrap();
		}
	}
}
