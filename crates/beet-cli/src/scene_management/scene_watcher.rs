//! Loading, marking, watching and reloading the active `beet.json` scene.

use crate::prelude::*;
use beet::prelude::*;

/// File name of the scene the CLI loads from the cwd.
pub const BEET_SCENE_FILE: &str = "beet.json";

/// Marks a root entity spawned from the active `beet.json` scene, so the whole
/// scene can be despawned wholesale on reload. Named to avoid clashing with
/// bevy's own `SceneRoot` (a handle to a scene asset).
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct BeetSceneRoot;

/// Triggered just before the active scene is despawned, so scene behaviours can
/// return to a resting state (eg stop a robot, close a connection) before their
/// entities are removed. An extension point: the CLI triggers it, loaded scenes
/// add observers.
#[derive(Event)]
pub struct ResetScene;

/// Holds the [`FsWatcher`] entity for the active `beet.json`.
#[derive(Resource)]
pub struct BeetSceneWatcher {
	/// Absolute path of the watched `beet.json`.
	pub path: AbsPathBuf,
	/// The entity carrying the [`FsWatcher`] component.
	pub entity: Entity,
}

/// Startup system: load `beet.json` from the cwd, or render [`SceneNotFound`]
/// and exit when it is absent.
pub fn load_beet_scene(world: &mut World) -> Result {
	let path = AbsPathBuf::new(BEET_SCENE_FILE)?;
	if !path.exists() {
		render_scene_not_found(world)?;
		world.write_message(AppExit::Success);
		return Ok(());
	}
	set_scene(world, &fs_ext::read_media(&path)?)?;
	spawn_watcher(world, path);
	Ok(())
}

/// Despawn the active scene (if any), then spawn the scene described by `media`,
/// marking each spawned root [`BeetSceneRoot`]. Route trees rebuild via the
/// [`WorldSerdeLoaded`] observer registered by [`RouterPlugin`].
fn set_scene(world: &mut World, media: &MediaBytes) -> Result {
	let existing = world
		.query_filtered::<Entity, With<BeetSceneRoot>>()
		.iter(world)
		.collect::<Vec<_>>();
	if !existing.is_empty() {
		world.trigger(ResetScene);
		existing
			.into_iter()
			.for_each(|entity| world.entity_mut(entity).despawn());
	}

	// roots are the spawned entities with no parent; mark them so the whole
	// scene can be despawned together on the next reload.
	WorldSerdeLoader::new(world)
		.load(media)?
		.into_iter()
		.filter(|entity| !world.entity(*entity).contains::<ChildOf>())
		.collect::<Vec<_>>()
		.into_iter()
		.for_each(|root| {
			world.entity_mut(root).insert(BeetSceneRoot);
		});
	Ok(())
}

/// Spawn an [`FsWatcher`] for `beet.json` and stash it on [`BeetSceneWatcher`].
///
/// The parent directory is watched (filtered to `beet.json`) rather than the
/// file itself, so atomic editor saves that swap the file's inode are still
/// picked up.
fn spawn_watcher(world: &mut World, path: AbsPathBuf) {
	let dir = path.parent().unwrap_or_else(|| path.clone());
	let watch_path = path.clone();
	let entity = world
		.spawn(FsWatcher::new(dir).with_filter(
			GlobFilter::default().with_include(&format!("*{BEET_SCENE_FILE}")),
		))
		.observe_any(move |ev: On<DirEvent>, mut commands: Commands| {
			if !ev.any(|event| event.path == watch_path) {
				return;
			}
			let path = watch_path.clone();
			commands.queue(move |world: &mut World| {
				match fs_ext::read_media(&path) {
					Ok(media) => {
						if let Err(err) = set_scene(world, &media) {
							cross_log_error!(
								"failed to reload {BEET_SCENE_FILE}: {err}"
							);
						}
					}
					// a transient read error (eg mid-save) leaves the current
					// scene in place; the next event reloads it.
					Err(err) => {
						cross_log_error!("{BEET_SCENE_FILE} unreadable: {err}")
					}
				}
			});
		})
		.id();
	world.insert_resource(BeetSceneWatcher { path, entity });
}


#[cfg(all(test, feature = "qrcode"))]
mod test {
	use crate::prelude::*;
	use beet::prelude::*;

	fn cli_world() -> World {
		(AsyncPlugin, RouterPlugin, CliCommandsPlugin).into_world()
	}

	/// The default CLI commands round-trip through world serde: a serialized
	/// router reloads with every command route discoverable, proving the
	/// reflectable markers reconstruct their path/behaviour from require hooks.
	#[beet_core::test]
	fn scene_round_trips() {
		let mut world = cli_world();
		let root = world
			.spawn((default_router(), children![
				RunWasm, BuildWasm, ExportPdf, QrCode
			]))
			.flush();
		let json = WorldSerdeSaver::new(&mut world)
			.with_entity_tree(root)
			.deny_component::<ParamsPartial>()
			.save(MediaType::Json)
			.unwrap();

		// reload into a fresh world, as the `beet` binary does on startup.
		let mut world = cli_world();
		let root = WorldSerdeLoader::new(&mut world)
			.load(&json)
			.unwrap()
			.into_iter()
			.find(|entity| !world.entity(*entity).contains::<ChildOf>())
			.unwrap();
		// the route tree rebuild runs through deferred commands.
		world.flush();
		let tree = world.entity(root).get::<RouteTree>().unwrap();
		tree.find(&["build-wasm"]).xpect_some();
		tree.find(&["export-pdf"]).xpect_some();
		tree.find(&["qrcode"]).xpect_some();
		tree.find(&["run-wasm", "some-binary"]).xpect_some();
	}
}
