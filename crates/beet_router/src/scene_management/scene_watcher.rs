//! Loading, watching and reloading a `beet.json` scene from the cwd: the host
//! side of scene management, where the scene *is* a file the process watches.

use crate::prelude::*;
use beet_core::prelude::*;

/// File name of the scene a CLI loads from the cwd.
pub const BEET_SCENE_FILE: &str = "beet.json";

/// Loads, watches and reloads the [`BEET_SCENE_FILE`] scene from the cwd.
///
/// On startup [`load_beet_scene`] looks for a `beet.json`: absent, it renders
/// [`SceneNotFound`] and exits; present, it loads the scene (marking each root
/// [`BeetSceneRoot`]) and installs an [`FsWatcher`] that swaps the scene whenever
/// the file changes.
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

/// Startup system: load `beet.json` from the cwd, or render [`SceneNotFound`]
/// and exit when it is absent.
///
/// Note that here we deliberately do not use a CliServer, as that would conflict
/// with any CliServer spawned by a scene. this step is intentionally minimal.
pub fn load_beet_scene(world: &mut World) -> Result {
	let path = AbsPathBuf::new(BEET_SCENE_FILE)?;
	if !path.exists() {
		render_scene_not_found(world)?;
		world.write_message(AppExit::Success);
		return Ok(());
	}
	set_scene(world, &fs_ext::read_media(&path)?, None)?;
	spawn_watcher(world, path);
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
						if let Err(err) = set_scene(world, &media, None) {
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
