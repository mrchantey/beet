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

/// Parked on a [`WorldAssetRoot`] built into a template subtree: it records the
/// pending dependency gating the build root's [`LoadTemplate`] until the scene's
/// [`WorldInstanceReady`] fires.
#[derive(Component)]
struct PendingScene {
	/// The template build root carrying the [`TemplatePending`] set.
	root: Entity,
	/// The dependency id parked on that root.
	id: PendingId,
}

/// On a [`WorldAssetRoot`] added during a template build, park a [`PendingId`] on
/// the build root and record it on the entity, so `LoadTemplate` waits for the
/// scene to spawn. A `WorldAssetRoot` added outside a build gates nothing.
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
		let id = world
			.entity_mut(root)
			.entry::<TemplatePending>()
			.or_default()
			.get_mut()
			.register();
		world.entity_mut(entity).insert(PendingScene { root, id });
	});
}

/// On a scene's [`WorldInstanceReady`], resolve the entity's [`PendingScene`] and
/// drain its root, firing [`LoadTemplate`] once nothing else is pending.
fn resolve_pending_scene(
	ready: On<WorldInstanceReady>,
	pending: Query<&PendingScene>,
	mut commands: Commands,
) {
	let entity = ready.entity;
	let Ok(&PendingScene { root, id }) = pending.get(entity) else {
		return;
	};
	commands.queue(move |world: &mut World| {
		// set up the scene's freshly-spawned (bare) AnimationPlayers BEFORE firing
		// LoadTemplate, so a `RunOnLoad`-started tree never out-races the
		// `init_animators` Update system for a player that lacks its graph handle
		// and transitions.
		init_scene_animators(world, entity);
		let mut root_entity = world.entity_mut(root);
		if let Some(mut pending) = root_entity.get_mut::<TemplatePending>() {
			pending.resolve(id);
		}
		drain_pending_dependencies(&mut root_entity);
		world.entity_mut(entity).remove::<PendingScene>();
	});
}

/// Copy the model root's [`AnimationGraphHandle`] onto, and add an
/// [`AnimationTransitions`] to, every bare [`AnimationPlayer`] the scene spawned
/// under `scene_root`, mirroring `init_animators` but eagerly at scene-ready time.
fn init_scene_animators(world: &mut World, scene_root: Entity) {
	let graph = world.get::<AnimationGraphHandle>(scene_root).cloned();
	// collect the spawned players in the scene subtree that are not yet set up.
	let mut players = Vec::new();
	let mut stack = vec![scene_root];
	while let Some(entity) = stack.pop() {
		if world.get::<AnimationPlayer>(entity).is_some()
			&& world.get::<AnimationTransitions>(entity).is_none()
		{
			players.push(entity);
		}
		if let Some(children) = world.get::<Children>(entity) {
			stack.extend(children.iter());
		}
	}
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
	fn defers_load_until_scene_ready() {
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
		for _ in 0..20 {
			app.update();
			if fired.get() {
				break;
			}
		}
		fired.get().xpect_true();
	}
}
