//! The host scene commands: load, clear, reset, dump and run a scene. Each is
//! symmetric — it either drives a *remote* scene-server device over HTTP, or
//! applies to the *current binary* (the host the CLI is part of), depending on
//! whether a device URL is configured:
//!
//! - a `--url` request param, else
//! - the `BEET_REMOTE_URL` env var, else
//! - no URL — the command applies to the local world.
//!
//! Serialized into a `beet.json` (see beet-cli's `default_cli` example) and
//! loaded by the `beet` binary, these turn the CLI into a scene controller:
//!
//! ```sh
//! beet load scenes/led-script.json   # POST a scene file (remote) or set_scene (local)
//! beet run led-script                # fire an action route the scene installed
//! beet dump                          # print the loaded scene as JSON
//! beet clear                         # despawn the scene + reset
//! beet reset                         # stop the hardware
//! ```
//!
//! [`ExportScene`] is the inverse: it serializes its own descendant scene to a
//! `beet.json`, used by the export examples to generate the CLI itself.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

extern crate alloc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

/// Registers the scene-command reflect types so a `beet.json` carrying them
/// round-trips: the loader reconstructs each command's path/behaviour from its
/// require hooks.
pub struct SceneCommandsPlugin;

impl Plugin for SceneCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<SceneLoad>()
			.register_type::<SceneClear>()
			.register_type::<SceneReset>()
			.register_type::<SceneDump>()
			.register_type::<SceneRun>();
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
/// router. `<path>` is greedy so a slash-bearing path is captured whole.
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
	let media = fs_ext::read_media(&path)?;
	match device_url(&cx.input) {
		Some(url) => {
			Request::post(format!("{url}/load"))
				.with_content_type(media.media_type().clone())
				.with_body(media.bytes())
				.send()
				.await
		}
		None => {
			cx.caller
				.with_world(move |world, caller| -> Result<Response> {
					let server = world.root_ancestor(caller);
					let roots = set_scene(world, &media, Some(server))?;
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
/// despawns the active scene from the world.
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
/// `beet run led-script`. Remote hits `/<route>`; local exchanges it against the
/// host router.
#[action(route = "run/:route", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneRun(cx: ActionContext<RequestParts>) -> Result<Response> {
	let route = cx.input.get_param("route").unwrap_or("").to_string();
	match device_url(&cx.input) {
		Some(url) => Request::get(format!("{url}/{route}")).send().await,
		None => {
			let host = cx
				.caller
				.with_world(|world, caller| world.root_ancestor(caller))
				.await?;
			cx.caller
				.world()
				.entity(host)
				.exchange(Request::get(route))
				.await
				.xok()
		}
	}
}

/// Serialize this action's descendant scene to a JSON file. Each child of
/// [`ExportScene`] is a root of the exported scene, collected and written via
/// [`WorldSerdeSaver::save_roots`]. The output path is the `--output` request
/// param, defaulting to [`BEET_SCENE_FILE`].
///
/// Mounted at the root path so its descendants' serialized [`PathPattern`]s carry
/// no prefix (an `export/…` ancestor would corrupt the loaded routes); run the
/// host with just `--output <path>`.
#[action(route = "", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn ExportScene(cx: ActionContext<RequestParts>) -> Result<Response> {
	let output =
		AbsPathBuf::new(cx.input.get_param("output").unwrap_or(BEET_SCENE_FILE))?;
	cx.caller
		.with_world(move |world, caller| -> Result<Response> {
			let roots = world
				.entity(caller)
				.get::<Children>()
				.map(|children| children.iter().collect::<Vec<_>>())
				.unwrap_or_default();
			let json =
				WorldSerdeSaver::save_roots(world, MediaType::Json, roots)?
					.as_utf8()?
					.to_string();
			fs_ext::write(&output, &json)?;
			Response::ok_text(format!("wrote scene to {output}\n")).xok()
		})
		.await?
}
