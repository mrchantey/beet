//! Remote scene control: a router-as-CLI that *fetches a device* running the
//! [`scene_server`](super::scene_server). Each route is a thin async action
//! around [`Request::send`] — the positional CLI args become the path, which the
//! router matches to a route, which issues an HTTP request to the device and
//! returns its reply.
//!
//! Serialize [`remote_scene`] to a `beet.json` (see beet-cli's `remote_loader`
//! example) and the `beet` binary becomes a remote control:
//!
//! ```sh
//! beet load scenes/led-script.json   # POST a scene file to the device /load
//! beet run led-script                # fire an action route the scene installed
//! beet dump                          # print the loaded scene as JSON
//! beet clear                         # despawn the scene + reset
//! beet reset                         # stop the hardware
//! ```
//!
//! The device URL comes from the `SCENE_URL` env var, falling back to
//! [`DEFAULT_DEVICE_URL`].

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Device URL used when `SCENE_URL` is unset.
pub const DEFAULT_DEVICE_URL: &str = "http://127.0.0.1:8080";

/// The device's base URL, from `SCENE_URL` or [`DEFAULT_DEVICE_URL`].
fn device_url() -> String {
	env_ext::var("SCENE_URL").unwrap_or_else(|_| DEFAULT_DEVICE_URL.to_string())
}

/// Registers the remote command reflect types so a `beet.json` carrying them
/// round-trips: the loader reconstructs each command's path/behaviour from its
/// require hooks.
pub struct RemoteCommandsPlugin;

impl Plugin for RemoteCommandsPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<RemoteLoad>()
			.register_type::<RemoteClear>()
			.register_type::<RemoteReset>()
			.register_type::<RemoteDump>()
			.register_type::<RemoteRun>();
	}
}

/// The remote-control router bundle: the [`load`](RemoteLoad)/[`clear`](RemoteClear)/
/// [`reset`](RemoteReset)/[`dump`](RemoteDump)/[`run`](RemoteRun) commands wired
/// under a [`default_router`]. Serialize this to a `beet.json` to make the `beet`
/// binary a remote control for a scene-server device.
pub fn remote_scene() -> impl Bundle {
	(default_router(), children![
		// `*scene` is greedy so a slash-bearing path like `scenes/led.json`
		// (which the CLI splits into segments) is captured whole.
		exchange_route("load/*scene", RemoteLoad),
		exchange_route("clear", RemoteClear),
		exchange_route("reset", RemoteReset),
		exchange_route("dump", RemoteDump),
		exchange_route("run/:route", RemoteRun),
	])
}

/// `load <path>` — POST a scene file to the device's `/load`, replacing its
/// current routes. `<path>` is a path to a scene JSON file.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn RemoteLoad(cx: ActionContext<RequestParts>) -> Result<Response> {
	let path = cx
		.input
		.get_params("scene")
		.map(|segments| segments.join("/"))
		.unwrap_or_default();
	if path.is_empty() {
		bevybail!("usage: load <path-to-scene.json>");
	}
	let body = fs_ext::read(&path)?;
	Request::post(format!("{}/load", device_url()))
		.with_content_type(MediaType::Json)
		.with_body(body)
		.send()
		.await
}

/// `clear` — despawn the loaded scene and reset the device.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn RemoteClear(_cx: ActionContext<RequestParts>) -> Result<Response> {
	Request::get(format!("{}/clear", device_url())).send().await
}

/// `reset` — return the hardware to its resting state.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn RemoteReset(_cx: ActionContext<RequestParts>) -> Result<Response> {
	Request::get(format!("{}/reset", device_url())).send().await
}

/// `dump` — print the currently loaded scene as JSON.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn RemoteDump(_cx: ActionContext<RequestParts>) -> Result<Response> {
	Request::get(format!("{}/dump", device_url())).send().await
}

/// `run <route>` — fire an action route the loaded scene installed, eg
/// `beet run led-script`.
#[action(handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
async fn RemoteRun(cx: ActionContext<RequestParts>) -> Result<Response> {
	let route = cx.input.get_param("route").unwrap_or("").to_string();
	Request::get(format!("{}/{route}", device_url()))
		.send()
		.await
}
