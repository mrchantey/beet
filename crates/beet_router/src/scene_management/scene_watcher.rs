//! Host-side scene retention: the `beet` CLI's built-in scene server and its
//! `.beet` cache.
//!
//! Unlike a device, the CLI is a one-shot process: each invocation loads its
//! retained scene, runs one command, then exits. [`start_server`] spawns the
//! host (a [`CliServer`] carrying the built-in scene commands) and rehydrates
//! the last scene from [`BEET_CACHE_PATH`]; [`persist_scene`] rewrites that cache
//! whenever the local scene changes, so state survives between invocations.
//!
//! A `beet load <file> --watch` additionally [`spawn_scene_watcher`]s an
//! [`FsWatcher`] on the file, reloading the scene (and rewriting the cache) on
//! every save. The watch is recorded by a reflectable [`SceneWatch`] marker so it
//! round-trips through the cache and is reattached on the next startup.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

extern crate alloc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

/// Relative path of the CLI's retained-scene cache, loaded on startup and
/// rewritten whenever the local scene changes.
pub const BEET_CACHE_PATH: &str = ".beet/scene.json";

/// Registers the host scene-management reflect types, shared by any binary that
/// embeds the scene commands. The server itself is spawned by [`start_server`],
/// not here, so downstream binaries keep control of their own startup.
pub struct SceneManagementPlugin;

impl Plugin for SceneManagementPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<BeetSceneRoot>()
			.register_type::<SceneWatch>()
			.add_plugins(SceneCommandsPlugin);
	}
}

/// Persisted marker recording that a loaded scene file should be watched for
/// changes. Reflectable so it round-trips through [`BEET_CACHE_PATH`]; the live
/// [`FsWatcher`] is (re)attached by [`spawn_scene_watcher`] on load.
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct SceneWatch {
	/// Path of the watched scene file.
	pub path: String,
}

/// Startup system: spawn the host CLI server with the built-in scene commands,
/// then rehydrate the retained scene from [`BEET_CACHE_PATH`] (if any).
///
/// The host is a single [`CliServer`] + [`default_router`]; scenes load *under*
/// it as route children, mirroring the device's [`HttpServer`] host. The
/// welcome page answers `/` until a scene supplies its own root route.
pub fn start_server(world: &mut World) -> Result {
	let host = world
		.spawn((CliServer, default_router(), children![
			SceneLoad,
			SceneClear,
			SceneReset,
			SceneDump,
			SceneRun,
		]))
		.id();
	// the welcome page at `/`, shown until a loaded scene overrides it.
	world.entity_mut(host).with_child(scene_not_found_route());

	let path = AbsPathBuf::new(BEET_CACHE_PATH)?;
	if path.exists() {
		set_scene(world, &fs_ext::read_media(&path)?, Some(host))?;
		reattach_watchers(world, host);
	}
	Ok(())
}

/// Persist the active scene to [`BEET_CACHE_PATH`] so the next CLI invocation
/// rehydrates it. Only the [`BeetSceneRoot`] trees are saved (not the built-in
/// server/commands); when no scene is loaded the cache file is removed.
pub fn persist_scene(world: &mut World) -> Result {
	let path = AbsPathBuf::new(BEET_CACHE_PATH)?;
	let has_roots = world
		.query_filtered::<Entity, With<BeetSceneRoot>>()
		.iter(world)
		.next()
		.is_some();
	if !has_roots {
		if path.exists() {
			fs_ext::remove(&path)?;
		}
		return Ok(());
	}
	let json =
		WorldSerdeSaver::save_roots_filtered::<With<BeetSceneRoot>>(
			world,
			MediaType::Json,
		)?
		.as_utf8()?
		.to_string();
	fs_ext::write(&path, &json)?;
	Ok(())
}

/// Spawn an [`FsWatcher`] on `path`, reloading the scene under `host` on every
/// save. The watcher entity is itself a [`BeetSceneRoot`] carrying a [`SceneWatch`]
/// marker, so it is cleared with the scene and persisted to the cache.
///
/// The parent directory is watched (filtered to the file name) rather than the
/// file itself, so atomic editor saves that swap the file's inode are still
/// picked up.
pub fn spawn_scene_watcher(world: &mut World, path: String, host: Entity) {
	let Ok(abs) = AbsPathBuf::new(&path) else {
		cross_log_error!("cannot watch missing path: {path}");
		return;
	};
	let dir = abs.parent().unwrap_or_else(|| abs.clone());
	let file_name = abs
		.file_name()
		.map(|name| name.to_string_lossy().to_string())
		.unwrap_or_default();
	let watch_path = abs.clone();
	world
		.spawn((
			BeetSceneRoot,
			SceneWatch { path: path.clone() },
			ChildOf(host),
			FsWatcher::new(dir).with_filter(
				GlobFilter::default().with_include(&format!("*{file_name}")),
			),
		))
		.observe_any(move |ev: On<DirEvent>, mut commands: Commands| {
			if !ev.any(|event| event.path == watch_path) {
				return;
			}
			let path = path.clone();
			commands.queue(move |world: &mut World| {
				if let Err(err) = reload_watched(world, path, host) {
					cross_log_error!("scene reload failed: {err}");
				}
			});
		});
}

/// Reload a watched scene file: swap the active scene, respawn its watcher and
/// rewrite the cache. A transient read error (eg mid-save) is logged and the
/// current scene left in place; the next event reloads it.
fn reload_watched(world: &mut World, path: String, host: Entity) -> Result {
	let media = fs_ext::read_media(&path)?;
	set_scene(world, &media, Some(host))?;
	spawn_scene_watcher(world, path, host);
	persist_scene(world)
}

/// Reattach a live [`FsWatcher`] to every [`SceneWatch`] rehydrated from the
/// cache: the marker round-trips through reflection but the watcher itself does
/// not, so each is despawned and respawned via [`spawn_scene_watcher`].
fn reattach_watchers(world: &mut World, host: Entity) {
	let watchers = world
		.query::<(Entity, &SceneWatch)>()
		.iter(world)
		.map(|(entity, watch)| (entity, watch.path.clone()))
		.collect::<Vec<_>>();
	for (entity, path) in watchers {
		world.entity_mut(entity).despawn();
		spawn_scene_watcher(world, path, host);
	}
}
