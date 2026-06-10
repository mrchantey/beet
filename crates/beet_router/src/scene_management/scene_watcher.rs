//! Host-side scene retention, wired entirely through ECS primitives.
//!
//! The CLI is a one-shot process: each invocation rehydrates its retained scene,
//! runs one command, then exits. Three pieces of machinery, all reactive:
//!
//! - [`rehydrate_scene_cache`]: a [`Startup`] system that loads the last scene
//!   from [`BEET_CACHE_PATH`] under the host. Because the cache stores any
//!   [`SceneWatch`] markers, rehydrating them re-fires the watcher hook, so no
//!   separate "reattach watchers" step is needed.
//! - [`SceneWatch`]: a component whose `on_add` hook installs an [`FsWatcher`] and
//!   a [`DirEvent`] observer, re-applying the scene on every save. It fires both
//!   on a fresh `beet load --watch` and on cache rehydration.
//! - [`persist_on_root_added`] / [`persist_on_root_removed`]: observers that keep
//!   [`BEET_CACHE_PATH`] in sync with the live [`BeetSceneRoot`] set, so state
//!   survives between invocations with no manual save calls.
//!
//! All of this is wired by [`SceneManagementPlugin::build`]; a downstream binary
//! just adds the plugin and spawns its host entity (see beet-cli's `main`).

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

extern crate alloc;
use alloc::string::ToString;

/// Workspace-relative path of the CLI's retained-scene cache, loaded on startup
/// and rewritten whenever the local [`BeetSceneRoot`] set changes.
///
/// Resolved against the workspace root (not the cwd) so the retained scene is a
/// project-level artifact: a `beet load` from the repo root and a later `beet`
/// invocation from a crate subdir (eg the `run-wasm` cargo test runner, which
/// runs from each package's directory) share one cache.
pub const BEET_CACHE_PATH: &str = ".beet/scene.json";

/// Wires the host scene-management machinery: reflect types, the cache-rehydrate
/// [`Startup`] system and the persistence observers. The host entity itself is
/// spawned by the downstream binary, so it keeps control of its own startup.
pub struct SceneManagementPlugin;

impl Plugin for SceneManagementPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<BeetSceneRoot>()
			.register_type::<SceneWatch>()
			.add_plugins(SceneCommandsPlugin)
			.add_systems(Startup, rehydrate_scene_cache)
			.add_observer(persist_on_root_added)
			.add_observer(persist_on_root_removed);
	}
}

/// Records that a loaded scene file should be watched for changes. Its `on_add`
/// hook installs the live [`FsWatcher`] + reload observer, so adding it (whether
/// from `beet load --watch` or rehydrated from [`BEET_CACHE_PATH`]) is enough to
/// start watching. Reflectable so it round-trips through the cache.
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = watch_scene_file)]
pub struct SceneWatch {
	/// Path of the watched scene file.
	pub path: AbsPathBuf,
}

/// Marks a [`SceneWatch`] that mirrors its file to a *remote* device: on every
/// save the file is POSTed to `<url>/load` rather than applied to the local
/// world. Not reflected, since a remote watch is a live process, never cached.
#[derive(Clone, Component)]
pub struct RemoteWatch {
	/// The device URL to POST the file to (without the trailing `/load`).
	pub url: Url,
}

/// `on_add` hook for [`SceneWatch`]: install an [`FsWatcher`] on the file's
/// parent directory (filtered to the file name, so atomic editor saves that swap
/// the inode are still seen) plus a [`DirEvent`] observer that reloads on save.
///
/// The watcher is added to the same entity, so the entity's own [`FsWatcher`]
/// `on_add` starts the OS watch; the observer reapplies the scene reactively
/// rather than passing the world around by hand.
fn watch_scene_file(mut world: DeferredWorld, cx: HookContext) {
	let entity = cx.entity;
	let path = world.entity(entity).get::<SceneWatch>().unwrap().path.clone();
	let dir = path.parent().unwrap_or_else(|| path.clone());
	let file_name = path
		.file_name()
		.map(|name| name.to_string_lossy().to_string())
		.unwrap_or_default();
	let watch_path = path.clone();

	world
		.commands()
		.entity(entity)
		.insert(FsWatcher::new(dir).with_filter(
			GlobFilter::default().with_include(&format!("*{file_name}")),
		))
		.observe_any(move |ev: On<DirEvent>, mut commands: Commands| {
			if !ev.any(|event| event.path == watch_path) {
				return;
			}
			commands.queue_async(move |world: AsyncWorld| async move {
				if let Err(err) = reload_watched(world, entity).await {
					cross_log_error!("scene reload failed: {err}");
				}
			});
		});
}

/// Re-apply a watched scene file after a save. A [`RemoteWatch`] re-uploads the
/// file (POST `<url>/load`); otherwise the scene is swapped into the local world
/// under its host. A transient read error (eg mid-save) is logged and the
/// current scene left in place; the next event reloads it.
async fn reload_watched(world: AsyncWorld, watcher: Entity) -> Result {
	let path = world
		.with(move |world| {
			world.entity(watcher).get::<SceneWatch>().map(|w| w.path.clone())
		})
		.await
		.ok_or_else(|| bevyhow!("watcher entity missing SceneWatch"))?;
	let media = fs_ext::read_media(&path)?;

	let remote = world
		.with(move |world| {
			world.entity(watcher).get::<RemoteWatch>().map(|r| r.url.clone())
		})
		.await;
	match remote {
		Some(url) => {
			Request::post(format!("{url}/load"))
				.with_content_type(media.media_type().clone())
				.with_body(media.bytes())
				.send()
				.await?;
			Ok(())
		}
		None => {
			world
				.with(move |world| -> Result {
					let host = world.root_ancestor(watcher);
					set_scene(world, &media, Some(host))?;
					Ok(())
				})
				.await
		}
	}
}

/// [`Startup`] system: rehydrate the retained scene from [`BEET_CACHE_PATH`]
/// under the single host. Any [`SceneWatch`] markers in the cache re-fire their
/// `on_add` hook, reattaching watchers automatically.
pub fn rehydrate_scene_cache(world: &mut World) -> Result {
	let path = AbsPathBuf::new_workspace_rel(BEET_CACHE_PATH)?;
	if !path.exists() {
		return Ok(());
	}
	let host = scene_host(world)?;
	set_scene(world, &fs_ext::read_media(&path)?, Some(host))?;
	Ok(())
}

/// The single host entity loaded scenes hang under: the lone [`CliServer`] (or
/// equivalent root). Errors if there is not exactly one, since a retained scene
/// has no unambiguous home otherwise.
fn scene_host(world: &mut World) -> Result<Entity> {
	let mut hosts = world.query_filtered::<Entity, With<CliServer>>();
	let mut iter = hosts.iter(world);
	let host = iter
		.next()
		.ok_or_else(|| bevyhow!("no host entity to load the retained scene"))?;
	if iter.next().is_some() {
		bevybail!("multiple host entities, cannot pick a retained-scene home");
	}
	Ok(host)
}

/// Observer: rewrite [`BEET_CACHE_PATH`] when a [`BeetSceneRoot`] is added, so a
/// freshly loaded scene survives the next invocation.
fn persist_on_root_added(_ev: On<Add, BeetSceneRoot>, mut commands: Commands) {
	commands.queue(|world: &mut World| {
		if let Err(err) = persist_scene_cache(world) {
			cross_log_error!("scene persist failed: {err}");
		}
	});
}

/// Observer: rewrite (or remove) [`BEET_CACHE_PATH`] when a [`BeetSceneRoot`] is
/// removed, so clearing a scene clears the cache.
fn persist_on_root_removed(
	_ev: On<Remove, BeetSceneRoot>,
	mut commands: Commands,
) {
	commands.queue(|world: &mut World| {
		if let Err(err) = persist_scene_cache(world) {
			cross_log_error!("scene persist failed: {err}");
		}
	});
}

/// Persist the active scene to [`BEET_CACHE_PATH`]: only the [`BeetSceneRoot`]
/// trees are saved (not the host server/commands); when no scene is loaded the
/// cache file is removed. Idempotent, so repeated calls during a swap are safe.
fn persist_scene_cache(world: &mut World) -> Result {
	let path = AbsPathBuf::new_workspace_rel(BEET_CACHE_PATH)?;
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
	let json = TemplateSaver::new()
		.save_roots_filtered::<With<BeetSceneRoot>>(world, MediaType::Json)?
		.as_utf8()?
		.to_string();
	fs_ext::write(&path, &json)?;
	Ok(())
}


#[cfg(all(test, feature = "template_serde", feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Spawning a [`SceneWatch`] installs an [`FsWatcher`] via its `on_add` hook,
	/// so no separate setup or reattach step is needed.
	#[beet_core::test]
	async fn scene_watch_hook_installs_watcher() {
		// a real file so the watcher's own `on_add` existence check passes.
		let dir = AbsPathBuf::new(std::env::temp_dir()).unwrap();
		let path = dir.join("beet_scene_watch_test.json");
		fs_ext::write(&path, "[]").unwrap();

		let mut world = AsyncPlugin.into_world();
		let entity = world.spawn(SceneWatch { path }).id();
		world.flush();
		world
			.entity(entity)
			.contains::<FsWatcher>()
			.xpect_true();
	}
}
