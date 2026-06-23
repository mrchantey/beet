//! The host scene-push commands: load, clear, reset, dump and run a scene on a
//! *remote* scene-server device over HTTP. Each targets a device by URL — a
//! `--url` request param, else the `BEET_REMOTE_URL` env var:
//!
//! ```sh
//! beet load scenes/led-script.json --url=http://device   # POST a scene file
//! beet run led-script --url=http://device                # fire a route the scene installed
//! beet dump --url=http://device                          # print the device's scene as JSON
//! beet clear --url=http://device                         # despawn the scene + reset
//! beet reset --url=http://device                         # return the hardware to rest
//! ```
//!
//! These are inert capabilities until a `main.bsx` wires them as routes (the
//! device-push side of the unified model). The receiving counterpart is
//! [`scene_server`](super::scene_server), which applies a pushed scene via
//! [`set_scene`](super::set_scene).

use beet_core::prelude::*;
use beet_net::prelude::*;

extern crate alloc;
use alloc::string::String;

/// Registers the scene-command reflect types so a scene carrying them round-trips:
/// the loader reconstructs each command's path/behaviour from its require hooks.
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
/// `BEET_REMOTE_URL` env var. Errors when neither is set, since these commands
/// only drive a remote device.
fn device_url(parts: &RequestParts) -> Result<String> {
	parts
		.get_param("url")
		.map(String::from)
		.or_else(|| env_ext::var("BEET_REMOTE_URL").ok())
		.ok_or_else(|| {
			bevyhow!("a device `--url` (or `BEET_REMOTE_URL`) is required")
		})
}

/// `load <path> --url=<device>` — POST a scene file to the device's `/load`.
/// `<path>` is greedy so a slash-bearing path is captured whole, and is read
/// through the nearest ancestor [`BlobStore`] (the workspace store in dev, S3 in a
/// deployed task), never the filesystem directly, so it works on every platform.
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
		bevybail!("usage: load <path-to-scene.json> --url=<device>");
	}
	let url = device_url(&cx.input)?;
	// read the scene through the nearest ancestor store (absent is an error).
	let store = cx
		.caller
		.with_state::<AncestorQuery<&BlobStore>, Result<BlobStore>>(
			|entity, stores| stores.get(entity).cloned(),
		)
		.await??;
	let media = store.get_media(&SmolPath::from(path.as_str())).await?;
	Request::post(format!("{url}/load"))
		.with_content_type(media.media_type().clone())
		.with_body(media.bytes())
		.send()
		.await?;
	Response::ok_text("uploaded scene\n").xok()
}

/// `clear --url=<device>` — despawn the device's scene and reset.
#[action(route = "clear", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneClear(cx: ActionContext<RequestParts>) -> Result<Response> {
	let url = device_url(&cx.input)?;
	Request::get(format!("{url}/clear")).send().await
}

/// `reset --url=<device>` — return the device hardware to its resting state.
#[action(route = "reset", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneReset(cx: ActionContext<RequestParts>) -> Result<Response> {
	let url = device_url(&cx.input)?;
	Request::get(format!("{url}/reset")).send().await
}

/// `dump --url=<device>` — print the device's currently loaded scene as JSON.
#[action(route = "dump", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneDump(cx: ActionContext<RequestParts>) -> Result<Response> {
	let url = device_url(&cx.input)?;
	Request::get(format!("{url}/dump")).send().await
}

/// `run <route> --url=<device>` — fire an action route the device's scene
/// installed, eg `beet run led-script`. The original request (method, headers,
/// query and body) is forwarded unchanged; only its destination URL is rewritten
/// to `<device>/<route>`.
#[action(route = "run/:route", handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub async fn SceneRun(cx: ActionContext<Request>) -> Result<Response> {
	let route = SmolStr::from(cx.input.get_param("route").unwrap_or(""));
	let url = device_url(cx.input.request_parts())?;
	let target = Url::parse(format!("{url}/{route}"));
	let (mut parts, body) = cx.input.into_parts();
	// redirect the request onto the device, keeping its query + fragment.
	*parts.url_mut() = parts.url().forward(&target);
	Request::from_parts(parts, body).send().await
}
