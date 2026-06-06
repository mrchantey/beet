//! Loading, watching and reloading a `beet.json` scene from the cwd: the host
//! side of scene management, where the scene *is* a file the process watches.
//!
//! The host is a single [`CliServer`] + [`default_router`]; the scene loads
//! *under* it, so a `beet.json` need only carry the command routes, not its own
//! server (symmetric with the device's [`HttpServer`] host). When no `beet.json`
//! exists the host serves the [`SceneNotFound`] welcome page at `/`.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// File name of the scene a CLI loads from the cwd.
pub const BEET_SCENE_FILE: &str = "beet.json";

/// Loads, watches and reloads the [`BEET_SCENE_FILE`] scene from the cwd.
///
/// On startup [`load_beet_scene`] spawns the host, then looks for a `beet.json`:
/// absent, it serves [`SceneNotFound`]; present, it loads the scene under the
/// host (marking each root [`BeetSceneRoot`]) and installs an [`FsWatcher`] that
/// swaps the scene whenever the file changes.
pub struct SceneManagementPlugin;

impl Plugin for SceneManagementPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<BeetSceneRoot>()
			.add_systems(Startup, load_beet_scene);
	}
}

/// Holds the [`FsWatcher`] entity for the active `beet.json`.
#[derive(Resource)]
pub struct BeetSceneWatcher {
	/// Absolute path of the watched `beet.json`.
	pub path: AbsPathBuf,
	/// The entity carrying the [`FsWatcher`] component.
	pub entity: Entity,
}

/// Startup system: spawn the host, then load `beet.json` from the cwd under it,
/// or serve [`SceneNotFound`] when it is absent.
pub fn load_beet_scene(world: &mut World) -> Result {
	// the single host: a CliServer parses the args, dispatches through the
	// router and exits. Scenes load under it as route children.
	let host = world.spawn((CliServer, default_router())).id();

	let path = AbsPathBuf::new(BEET_SCENE_FILE)?;
	if path.exists() {
		set_scene(world, &fs_ext::read_media(&path)?, Some(host))?;
		spawn_watcher(world, path, host);
	} else {
		// no beet.json: serve the welcome page at the root path.
		world.entity_mut(host).with_child(scene_not_found_route());
	}
	Ok(())
}

/// Spawn an [`FsWatcher`] for `beet.json` and stash it on [`BeetSceneWatcher`].
///
/// The parent directory is watched (filtered to `beet.json`) rather than the
/// file itself, so atomic editor saves that swap the file's inode are still
/// picked up. Reloads swap the scene under `host`.
fn spawn_watcher(world: &mut World, path: AbsPathBuf, host: Entity) {
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
						if let Err(err) = set_scene(world, &media, Some(host)) {
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
