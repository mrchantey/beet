use beet_core::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use bevy::world_serialization::WorldInstanceReady;

/// Defers a template's [`LoadTemplate`] until a [`WorldAssetRoot`] it builds has
/// spawned its entities.
///
/// A `WorldAssetRoot` (eg a glb world scene) spawns its children
/// asynchronously, so there is a window where the root exists but its
/// `AnimationPlayer`/colliders/etc do not. This plugin parks a pending
/// dependency on the build root when the `WorldAssetRoot` is built, and resolves
/// it on the scene's [`WorldInstanceReady`], so a load verb only runs once the
/// spawned children are guaranteed present. Mirrors the asset deferral
/// ([`AssetLoadTemplate`]/`drain_loaded_assets`), gating on a real signal rather
/// than a per-frame `Added<AnimationPlayer>` heuristic.
#[derive(Default)]
pub struct SceneReadyPlugin;

impl Plugin for SceneReadyPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(register_pending_scene)
			.add_observer(resolve_pending_scene);
	}
}

/// On a [`WorldAssetRoot`] added during a template build, park a
/// [`PendingDependency`] on the entity (its guard on the build root), so
/// `LoadTemplate` waits for the scene to spawn; a despawned scene resolves it
/// implicitly. A `WorldAssetRoot` added outside a build gates nothing.
fn register_pending_scene(
	add: On<Add, WorldAssetRoot>,
	build_root: Option<Res<TemplateBuildRoot>>,
	mut commands: Commands,
) {
	let Some(build_root) = build_root else {
		return;
	};
	let entity = add.entity;
	let root = **build_root;
	// register before this build's `drain_pending_dependencies`: the queue drains
	// at the next world sync, ahead of the root's synchronous drain.
	commands.queue(move |world: &mut World| {
		let guard =
			TemplatePending::park_on(world, root, PendingKind::Passive);
		let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
			// scene entity gone before the command ran: the dropped guard
			// resolves through the sweep.
			return;
		};
		entity_mut.insert(PendingDependency::new(guard));
	});
}

/// On a scene's [`WorldInstanceReady`], remove the entity's
/// [`PendingDependency`] (which resolves it), firing [`LoadTemplate`] once
/// nothing else is pending on the root.
fn resolve_pending_scene(
	ready: On<WorldInstanceReady>,
	pending: Query<(), With<PendingDependency>>,
	mut commands: Commands,
) {
	let entity = ready.entity;
	if !pending.contains(entity) {
		return;
	}
	commands.queue(move |world: &mut World| {
		// set up the scene's freshly-spawned (bare) AnimationPlayers BEFORE firing
		// LoadTemplate, so a `CallOnLoad`-started tree never out-races the
		// `init_animators` Update system for a player that lacks its graph handle
		// and transitions. The remove resolves the dependency via its hook, whose
		// queued drain runs after this command.
		init_scene_animators(world, entity);
		world.entity_mut(entity).remove::<PendingDependency>();
	});
}

/// Copy the model root's [`AnimationGraphHandle`] onto, and add an
/// [`AnimationTransitions`] to, every bare [`AnimationPlayer`] the scene spawned
/// under `scene_root`, mirroring `init_animators` but eagerly at scene-ready time.
fn init_scene_animators(world: &mut World, scene_root: Entity) {
	let graph = world.get::<AnimationGraphHandle>(scene_root).cloned();
	// the spawned players in the scene subtree that are not yet set up.
	let subtree = world.entity_mut(scene_root).iter_descendents_inclusive();
	let players = subtree
		.into_iter()
		.filter(|&entity| {
			world.get::<AnimationPlayer>(entity).is_some()
				&& world.get::<AnimationTransitions>(entity).is_none()
		})
		.collect::<Vec<_>>();
	for player in players {
		let mut entity = world.entity_mut(player);
		// bevy's animation systems skip a player lacking its graph handle.
		if let Some(graph) = &graph {
			entity.insert(graph.clone());
		}
		entity.insert(AnimationTransitions::new());
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::asset::AssetPlugin;
	use bevy::asset::Assets;
	use bevy::world_serialization::WorldAsset;
	use bevy::world_serialization::WorldSerializationPlugin;

	#[beet_core::test]
	async fn defers_load_until_scene_ready() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			WorldSerializationPlugin,
			TemplatePlugin,
			SceneReadyPlugin,
		));

		let fired = Store::new(false);
		let f = fired.clone();
		app.world_mut()
			.add_observer(move |_: On<LoadTemplate>| f.set(true));

		// a minimal world scene asset, added directly so it is immediately available.
		let mut asset_world = World::new();
		asset_world.spawn_empty();
		let handle = app
			.world_mut()
			.resource_mut::<Assets<WorldAsset>>()
			.add(WorldAsset::new(asset_world));

		// build a template hosting the `WorldAssetRoot`, as a scene template would.
		app.world_mut()
			.spawn_template(Snippet::from_bundle(WorldAssetRoot(handle)))
			.unwrap();

		// LoadTemplate deferred: the scene has not spawned (WorldInstanceReady) yet.
		fired.get().xpect_false();

		// drive the spawner until it spawns the instance and fires WorldInstanceReady.
		app_ext::update_until(&mut app, |_world| fired.get())
			.await
			.xpect_true();
	}
}
