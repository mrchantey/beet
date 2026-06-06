//! The scene server: a bootstrap HTTP API whose *real* routes arrive over the
//! wire as a beet scene. The meta-routes here ([`LoadScene`], [`ClearScene`],
//! [`Reset`], [`DumpScene`], [`Home`]) load, swap and inspect that scene; the
//! behaviours it wires ([`ActionRoute`] and any domain leaves) live elsewhere.
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
use alloc::vec::Vec;

/// Registers the reflectable types every scene server understands ([`ActionRoute`]
/// and the [`BeetSceneRoot`] marker), so reflection can (de)serialize a loaded
/// scene. Domain crates add their own route/action/scene types on top.
pub struct SceneServerPlugin;

impl Plugin for SceneServerPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<ActionRoute>()
			.register_type::<BeetSceneRoot>();
	}
}

/// Wires an HTTP path to a behaviour tree. The tree is the route entity's single
/// child; calling the route spawns a detached task that runs it, then returns at
/// once. A scene supplies the path and the tree under it.
#[action(route, handler_only)]
#[derive(Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[type_path = "scene"]
pub async fn ActionRoute(cx: ActionContext<RequestParts>) -> Response {
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
					cross_log_error!("scene: failed to load: {err}");
					Response::status_text(
						StatusCode::BAD_REQUEST,
						format!("invalid scene: {err}\n"),
					)
				}
			}
		})
		.await
		.unwrap_or_else(|err| {
			cross_log_error!("scene: load failed: {err}");
			Response::status_text(
				StatusCode::INTERNAL_SERVER_ERROR,
				"scene load failed\n",
			)
		})
}

/// `GET /clear` — despawn the loaded scene and reset the hardware.
#[action(handler_only)]
#[derive(Default, Clone, Component)]
pub async fn ClearScene(cx: ActionContext<RequestParts>) -> Response {
	cx.caller
		.with_world(|world, caller| {
			despawn_scene(world);
			// rebuild the tree so the cleared routes drop out of dispatch.
			let server = world.root_ancestor(caller);
			world
				.run_system_cached_with(RouteTree::rebuild, server)
				.unwrap_or(Ok(()))
				.ok();
		})
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
			let roots = world
				.query_filtered::<Entity, With<BeetSceneRoot>>()
				.iter(world)
				.collect::<Vec<_>>();
			let mut saver = WorldSerdeSaver::new(world);
			for root in roots {
				saver = saver.with_entity_tree(root);
			}
			match saver
				.save(MediaType::Json)
				.and_then(|bytes| bytes.as_utf8().map(String::from))
			{
				Ok(json) => Response::ok_body(json, MediaType::Json),
				Err(err) => {
					cross_log_error!("scene: dump failed: {err}");
					Response::status_text(
						StatusCode::INTERNAL_SERVER_ERROR,
						format!("dump failed: {err}\n"),
					)
				}
			}
		})
		.await
		.unwrap_or_else(|err| {
			cross_log_error!("scene: dump failed: {err}");
			Response::status_text(
				StatusCode::INTERNAL_SERVER_ERROR,
				"dump failed\n",
			)
		})
}

/// `GET /` — list the meta-routes and the currently loaded scene routes.
#[action(handler_only)]
#[derive(Default, Clone, Component)]
pub async fn Home(cx: ActionContext<RequestParts>) -> Response {
	let routes = cx
		.caller
		.with_state::<AncestorQuery<&RouteTree>, String>(move |entity, query| {
			query
				.get(entity)
				.map(|tree| {
					tree.flatten()
						.iter()
						.map(|pattern| {
							format!("  /{}\n", pattern.annotated_path())
						})
						.collect::<String>()
				})
				.unwrap_or_default()
		})
		.await
		.unwrap_or_default();
	Response::ok_text(format!(
		"scene server\n\nmeta routes:\n  /load   POST a scene (json|postcard)\n  /clear  despawn scene + reset\n  /reset  stop hardware\n  /dump   current scene as json\n\nactive routes:\n{routes}"
	))
}
