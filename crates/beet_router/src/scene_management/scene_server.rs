//! The scene server: a bootstrap HTTP API whose *real* routes arrive over the
//! wire as a beet scene. The meta-routes here ([`LoadScene`], [`ClearScene`],
//! [`Reset`], [`DumpScene`]) load, swap and inspect that scene; the behaviours
//! it wires ([`SpawnAction`] and any domain leaves) live elsewhere. The route
//! listing / help is handled by the router's default not-found middleware, so
//! there is no bespoke home route.
//!
//! Hardware-agnostic and no_std-friendly, so the same server runs on a host or
//! on bare-metal firmware. Add [`SceneServerPlugin`] to register the reflectable
//! types a scene can carry; spawn the meta-routes under an [`HttpServer`] (or any
//! [`Router`]) to expose them.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

extern crate alloc;
use alloc::format;
use alloc::string::String;

/// Registers the reflectable types every scene server understands ([`SpawnAction`]
/// and the [`BeetSceneRoot`] marker) plus the [`SceneServer`] meta-route template,
/// so reflection can (de)serialize a loaded scene and markup can spawn the server.
/// Domain crates add their own route/action/scene types on top.
pub struct SceneServerPlugin;

impl Plugin for SceneServerPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<SpawnAction>()
			.register_type::<BeetSceneRoot>()
			.register_template::<SceneServer>();
	}
}

/// The scene-server meta-routes as a markup-spawnable bundle: place
/// `<SceneServer/>` under a `<Router {(HttpServer, BootOnLoad)}>` to expose
/// `POST /load`, `GET /clear`, `GET /reset` and `GET /dump` — the device side of
/// a scene push, receiving a scene over the wire and swapping it via
/// [`set_scene`]. The host side is the `SceneLoad`/`SceneClear`/... push commands.
#[template]
pub fn SceneServer() -> impl Bundle {
	(
		OnSpawn::insert_child(exchange_route("load", LoadScene)),
		OnSpawn::insert_child(exchange_route("clear", ClearScene)),
		OnSpawn::insert_child(exchange_route("reset", Reset)),
		OnSpawn::insert_child(exchange_route("dump", DumpScene)),
	)
}

/// Wires an HTTP path to a behaviour tree. The tree is the route entity's single
/// child; calling the route spawns a detached task that runs it, then returns at
/// once. A scene supplies the path and the tree under it.
#[action(route, handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[type_path = "scene"]
pub async fn SpawnAction(cx: ActionContext<RequestParts>) -> Response {
	let caller = cx.caller.clone();
	let child = caller
		.get(|children: &Children| children.first().copied())
		.await
		.ok()
		.flatten();
	match child {
		Some(child) => {
			// fire-and-forget: drive the tree to completion on the local pool so
			// the HTTP response returns immediately even for endless loops.
			let world = caller.world();
			world
				.run_async_local(move |world: AsyncWorld| async move {
					world.entity(child).call::<(), Outcome>(()).await?;
					Result::Ok(())
				})
				.await;
			Response::ok_text("action started\n")
		}
		None => Response::ok_text("no behaviour to run\n"),
	}
}

/// `POST /load` — load a scene from the request body (JSON or postcard, per the
/// `content-type`), replacing any previously loaded scene. The new roots are
/// reparented under the server so the router serves them as routes.
#[action(handler_only)]
#[derive(Default, Clone, Component)]
pub async fn LoadScene(cx: ActionContext<Request>) -> Response {
	let media = match cx.input.into_media_bytes().await {
		Ok(media) => media,
		Err(err) => {
			return Response::status_text(
				StatusCode::BAD_REQUEST,
				format!("failed to read scene body: {err}\n"),
			);
		}
	};

	cx.caller
		.with_world(move |world, caller| -> Response {
			let server = world.root_ancestor(caller);
			match set_scene(world, &media, Some(server)) {
				Ok(roots) => Response::ok_text(format!(
					"loaded scene: {} root(s)\n",
					roots.len()
				)),
				Err(err) => {
					error!("scene: failed to load: {err}");
					Response::status_text(
						StatusCode::BAD_REQUEST,
						format!("invalid scene: {err}\n"),
					)
				}
			}
		})
		.await
		.unwrap_or_else(|err| {
			error!("scene: load failed: {err}");
			Response::status_text(
				StatusCode::INTERNAL_SERVER_ERROR,
				"scene load failed\n",
			)
		})
}

/// `GET /clear` — despawn the loaded scene and reset the hardware. The route tree
/// is rebuilt by [`despawn_scene`], so the cleared routes drop out of dispatch.
#[action(handler_only)]
#[derive(Default, Clone, Component)]
pub async fn ClearScene(cx: ActionContext<RequestParts>) -> Response {
	cx.caller
		.with_world(|world, _caller| despawn_scene(world))
		.await
		.ok();
	Response::ok_text("scene cleared\n")
}

/// `GET /reset` — return the hardware to its resting state (motors stopped, LEDs
/// off), leaving any loaded scene in place.
#[action(handler_only)]
#[derive(Default, Clone, Component)]
pub async fn Reset(cx: ActionContext<RequestParts>) -> Response {
	cx.caller
		.with_world(|world, _caller| {
			world.trigger(ResetScene);
		})
		.await
		.ok();
	Response::ok_text("reset\n")
}

/// `GET /dump` — serialize the currently loaded scene (the [`BeetSceneRoot`]
/// trees) back to JSON. Empty when no scene is loaded.
#[action(handler_only)]
#[derive(Default, Clone, Component)]
pub async fn DumpScene(cx: ActionContext<RequestParts>) -> Response {
	cx.caller
		.with_world(|world, _caller| -> Response {
			match TemplateSaver::new()
				.save_roots_filtered::<With<BeetSceneRoot>>(
					world,
					MediaType::Json,
				)
				.and_then(|bytes| bytes.as_utf8().map(String::from))
			{
				Ok(json) => Response::ok_body(json, MediaType::Json),
				Err(err) => {
					error!("scene: dump failed: {err}");
					Response::status_text(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("dump failed: {err}\n"),
					)
				}
			}
		})
		.await
		.unwrap_or_else(|err| {
			error!("scene: dump failed: {err}");
			Response::status_text(
				StatusCode::INTERNAL_SERVER_ERROR,
				"dump failed\n",
			)
		})
}

#[cfg(all(test, feature = "json", feature = "rhai"))]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn server_world() -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		world
	}

	/// The device side of a scene push end to end: a host serializes a one-route
	/// scene, POSTs it to the server's `/load`, and the route it carried answers
	/// live — the server received the bytes, swapped them in via `set_scene`, and
	/// now dispatches the pushed route.
	///
	/// The route is an `TransformExchangeScript`: its reflectable component re-derives its
	/// runtime dispatch (`DispatchExchange`) from its `#[require]` hook on load, so it
	/// survives the round-trip (a bare `exchange_route`'s `DispatchExchange` does not,
	/// the scene-routing constraint a device scene authors around).
	#[beet_core::test(timeout_ms = 10000)]
	async fn load_route_installs_pushed_scene() {
		// the host builds + serializes a one-route scripted scene.
		let mut host = server_world();
		let root = host
			.spawn((
				Script::<(), String>::rhai(r#""pong""#),
				TransformExchangeScript::<(), String>::default(),
				PathPartial::new("ping"),
			))
			.flush();
		let scene = TemplateSaver::new()
			.with_entity_tree(&host, root)
			.save(&host, MediaType::Json)
			.unwrap();

		// the device runs the `SceneServer` meta-routes; POST the scene to /load.
		let mut world = server_world();
		let server = world
			.spawn((default_router(), children![exchange_route(
				"load", LoadScene
			)]))
			.flush();
		world
			.entity_mut(server)
			.exchange(
				Request::post("load")
					.with_content_type(MediaType::Json)
					.with_body(scene.bytes()),
			)
			.await
			.status()
			.xpect_eq(StatusCode::OK);
		world.flush();

		// the device installed the pushed route into its live route tree,
		world
			.entity(server)
			.get::<RouteTree>()
			.unwrap()
			.find(&["ping"])
			.xpect_some();
		// and dispatches it: the pushed route answers on the device.
		world
			.entity_mut(server)
			.exchange(Request::get("ping"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("pong");
	}
}
