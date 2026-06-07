//! The host scene commands: load, clear, reset, dump and run a scene. Each is
//! symmetric — it either drives a *remote* scene-server device over HTTP, or
//! applies to the *current binary* (the host the CLI is part of), depending on
//! whether a device URL is configured:
//!
//! - a `--url` request param, else
//! - the `BEET_REMOTE_URL` env var, else
//! - no URL — the command applies to the local world.
//!
//! These are built into the `beet` CLI (see beet-cli's `main`), turning it into
//! a scene controller:
//!
//! ```sh
//! beet load scenes/led-script.json   # POST a scene file (remote) or set_scene (local)
//! beet load scenes/led-script.json --watch  # local: reload on every save
//! beet run led-script                # fire an action route the scene installed
//! beet dump                          # print the loaded scene as JSON
//! beet clear                         # despawn the scene + reset
//! beet reset                         # stop the hardware
//! ```
//!
//! [`ExportScene`] is the inverse: it serializes the scene rooted at its entity
//! to a JSON file, used by the export examples to generate loadable scenes.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

extern crate alloc;
use alloc::string::String;
use alloc::string::ToString;

/// Registers the scene-command reflect types so a scene carrying them round-trips:
/// the loader reconstructs each command's path/behaviour from its require hooks.
pub struct SceneCommandsPlugin;

impl Plugin for SceneCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<SceneLoad>()
			.register_type::<SceneClear>()
			.register_type::<SceneReset>()
			.register_type::<SceneDump>()
			.register_type::<SceneRun>()
			.register_type::<ExportScene>()
			.register_type::<ExportPath>();
	}
}

/// The device URL a command targets: the `--url` request param, else the
/// `BEET_REMOTE_URL` env var. `None` means apply to the local world.
fn device_url(parts: &RequestParts) -> Option<String> {
	parts
		.get_param("url")
		.map(String::from)
		.or_else(|| env_ext::var("BEET_REMOTE_URL").ok())
}

/// `load <path>` — load a scene file. With a device URL, POSTs it to the
/// device's `/load`; otherwise loads it into the local world under the host
/// router (persisted reactively to [`BEET_CACHE_PATH`]). `<path>` is greedy so a
/// slash-bearing path is captured whole.
///
/// `--watch` reloads on every save and keeps the process alive: locally it
/// re-applies the scene, remotely it re-uploads (POST) the file to the device.
/// The watch is set up by spawning a [`SceneWatch`] entity, whose `on_add` hook
/// installs the watcher.
#[action(route = "load/*scene", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneLoad(cx: ActionContext<RequestParts>) -> Result<Response> {
	let path = cx
		.input
		.get_params("scene")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if path.is_empty() {
		bevybail!("usage: load <path-to-scene.json>");
	}
	let watch = cx.input.has_param("watch");
	let path = AbsPathBuf::new(&path)?;
	let media = fs_ext::read_media(&path)?;
	match device_url(&cx.input) {
		Some(url) => {
			Request::post(format!("{url}/load"))
				.with_content_type(media.media_type().clone())
				.with_body(media.bytes())
				.send()
				.await?;
			if watch {
				let url = Url::parse(url);
				cx.caller
					.with_world(move |world, caller| {
						let host = world.root_ancestor(caller);
						world.spawn((
							SceneWatch { path },
							RemoteWatch { url },
							ChildOf(host),
						));
						// keep the schedule running so the watcher can fire.
						world.insert_resource(KeepAlive);
					})
					.await?;
			}
			Response::ok_text("uploaded scene\n").xok()
		}
		None => {
			cx.caller
				.with_world(move |world, caller| -> Result<Response> {
					let host = world.root_ancestor(caller);
					let roots = set_scene(world, &media, Some(host))?;
					if watch {
						// a [`BeetSceneRoot`] so it is cleared with the scene and
						// persisted to the cache (reattaching on the next startup).
						world.spawn((
							BeetSceneRoot,
							SceneWatch { path },
							ChildOf(host),
						));
						// keep the schedule running so the watcher can fire.
						world.insert_resource(KeepAlive);
					}
					Response::ok_text(format!(
						"loaded scene: {} root(s)\n",
						roots.len()
					))
					.xok()
				})
				.await?
		}
	}
}

/// `clear` — despawn the loaded scene and reset. Remote hits `/clear`; local
/// despawns the active scene, which reactively clears [`BEET_CACHE_PATH`] via the
/// [`BeetSceneRoot`] remove observer.
#[action(route = "clear", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneClear(cx: ActionContext<RequestParts>) -> Result<Response> {
	match device_url(&cx.input) {
		Some(url) => Request::get(format!("{url}/clear")).send().await,
		None => {
			cx.caller
				.with_world(|world, _caller| despawn_scene(world))
				.await?;
			Response::ok_text("scene cleared\n").xok()
		}
	}
}

/// `reset` — return the hardware to its resting state. Remote hits `/reset`;
/// local triggers [`ResetScene`].
#[action(route = "reset", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneReset(cx: ActionContext<RequestParts>) -> Result<Response> {
	match device_url(&cx.input) {
		Some(url) => Request::get(format!("{url}/reset")).send().await,
		None => {
			cx.caller
				.with_world(|world, _caller| world.trigger(ResetScene))
				.await?;
			Response::ok_text("reset\n").xok()
		}
	}
}

/// `dump` — print the currently loaded scene as JSON. Remote hits `/dump`;
/// local serializes the [`BeetSceneRoot`] trees.
#[action(route = "dump", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneDump(cx: ActionContext<RequestParts>) -> Result<Response> {
	match device_url(&cx.input) {
		Some(url) => Request::get(format!("{url}/dump")).send().await,
		None => {
			cx.caller
				.with_world(|world, _caller| -> Result<Response> {
					let json = WorldSerdeSaver::save_roots_filtered::<
						With<BeetSceneRoot>,
					>(world, MediaType::Json)?
					.as_utf8()?
					.to_string();
					Response::ok_body(json, MediaType::Json).xok()
				})
				.await?
		}
	}
}

/// `run <route>` — fire an action route the loaded scene installed, eg
/// `beet run led-script`. The original request (method, headers, query params
/// and body) is forwarded unchanged; only its destination URL is rewritten —
/// to `<device>/<route>` for a remote, or the bare `<route>` against the host
/// router for local.
#[action(route = "run/:route", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneRun(cx: ActionContext<Request>) -> Result<Response> {
	let route = cx.input.get_param("route").unwrap_or("").to_string();
	match device_url(cx.input.request_parts()) {
		Some(url) => {
			let target = Url::parse(format!("{url}/{route}"));
			let (mut parts, body) = cx.input.into_parts();
			// redirect the request onto the device, keeping its query + fragment.
			*parts.url_mut() = parts.url().forward(&target);
			Request::from_parts(parts, body).send().await
		}
		None => {
			let host = cx
				.caller
				.with_world(|world, caller| world.root_ancestor(caller))
				.await?;
			let (mut parts, body) = cx.input.into_parts();
			*parts.url_mut() = parts.url().clone().with_path(vec![route]);
			cx.caller
				.world()
				.entity(host)
				.exchange(Request::from_parts(parts, body))
				.await
				.xok()
		}
	}
}

/// Output path baked onto an [`ExportScene`], so it needs no `--output` flag.
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct ExportPath(pub String);

/// Serialize a scene to a JSON file. [`ExportScene`] is the *export instruction*,
/// kept on a parent so it never pollutes the output; its single child is the
/// actual scene root that gets serialized (entity + descendants) via
/// [`WorldSerdeSaver::save_roots`]. The output path is the caller's
/// [`ExportPath`] component, else the `--output` request param.
///
/// Mounted at the root path so the exported root's [`PathPattern`]s carry no
/// prefix (an `export/…` ancestor would corrupt the loaded routes).
#[action(route = "", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn ExportScene(cx: ActionContext<RequestParts>) -> Result<Response> {
	let output = cx.input.get_param("output").map(String::from);
	cx.caller
		.with_world(move |world, caller| -> Result<Response> {
			Response::ok_text(export_entity(world, caller, output.as_deref())?)
				.xok()
		})
		.await?
}

/// Serialize the [`ExportScene`] entity's single child (the scene root) and its
/// descendants to a JSON file, resolving the path from the entity's
/// [`ExportPath`] component or the `default_output` fallback. The scene root is
/// the child rather than the entity itself, so the [`ExportScene`]/[`ExportPath`]
/// markers stay out of the exported scene.
fn export_entity(
	world: &mut World,
	entity: Entity,
	default_output: Option<&str>,
) -> Result<String> {
	let output = world
		.entity(entity)
		.get::<ExportPath>()
		.map(|path| path.0.clone())
		.or_else(|| default_output.map(String::from))
		.ok_or_else(|| {
			bevyhow!("no export path: set --output or an ExportPath component")
		})?;
	let output = AbsPathBuf::new(output)?;
	let root = world
		.entity(entity)
		.get::<Children>()
		.and_then(|children| children.iter().next())
		.ok_or_else(|| {
			bevyhow!("ExportScene has no child scene root to export")
		})?;
	let json = WorldSerdeSaver::save_roots(world, MediaType::Json, [root])?
		.as_utf8()?
		.to_string();
	fs_ext::write(&output, &json)?;
	Ok(format!("wrote scene to {output}\n"))
}
