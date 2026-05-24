//! Per-request scene route adapters.
//!
//! File-based page handlers return scene content as an `impl Bundle`, but the
//! router dispatches actions whose output must be [`ExchangeRouteOut`]. These
//! adapters bridge the two: a handler is turned into an `Action<In, SceneEntity>`
//! that is called on every request, runs the handler to produce a bundle,
//! spawns it as an ephemeral [`Document`] entity, and hands it back as a
//! renderable [`SceneEntity`].
//!
//! There is one adapter per handler kind, mirroring the [`Action`] constructors
//! (`new_pure`/`new_async`/`new_system`). Codegen selects the adapter by
//! inspecting the handler signature:
//!
//! - `fn() -> impl Bundle` — no context, static page; emitted via
//!   [`fixed_scene`](crate::prelude::fixed_scene) (spawned once), not these adapters.
//! - `fn(cx: ActionContext<In>) -> impl Bundle` — [`scene_pure`].
//! - `async fn(cx: ActionContext<In>) -> impl Bundle` — [`scene_async`].
//! - `fn(cx: In<ActionContext<In>>, ..system params) -> impl Bundle` — [`scene_system`].
//!
//! For pages whose content depends on the request, `In` is typically [`Request`]
//! (the full request, the same shape route scene actions like `CallerScene` use).

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;
use bevy::ecs::system::In;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;

/// Spawns handler-produced scene content as an ephemeral [`Document`] entity
/// and returns it as a [`SceneEntity`]. The entity is despawned after render.
async fn spawn_scene_bundle<B: 'static + Send + Bundle>(
	caller: AsyncEntity,
	bundle: B,
) -> Result<SceneEntity> {
	let id = caller
		.world()
		.with_then(move |world: &mut World| {
			world.spawn((Document::default(), bundle)).id()
		})
		.await;
	Ok(SceneEntity::new_ephemeral(id))
}

/// The shared second half of every scene adapter: takes the bundle produced by
/// a handler and renders it as an ephemeral [`SceneEntity`].
fn spawn_scene_step<B: 'static + Send + Sync + Bundle>() -> Action<B, SceneEntity>
{
	Action::new_async(|cx: ActionContext<B>| async move {
		spawn_scene_bundle(cx.caller, cx.input).await
	})
}

/// Adapts a sync page handler `fn(ActionContext<In>) -> impl Bundle` into a
/// per-request scene route action.
pub fn scene_pure<Func, Input, B>(handler: Func) -> Action<Input, SceneEntity>
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> B,
	Input: 'static + Send + Sync,
	B: 'static + Send + Sync + Bundle,
{
	Action::new_pure(handler).chain(spawn_scene_step::<B>())
}

/// Adapts an async page handler `async fn(ActionContext<In>) -> impl Bundle`
/// into a per-request scene route action.
pub fn scene_async<Func, Input, Fut, B>(
	handler: Func,
) -> Action<Input, SceneEntity>
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> Fut,
	Fut: 'static + MaybeSend + Future<Output = B>,
	Input: 'static + Send + Sync,
	B: 'static + Send + Sync + Bundle,
{
	Action::new_async(handler).chain(spawn_scene_step::<B>())
}

/// Adapts a system page handler `fn(In<ActionContext<In>>, ..) -> impl Bundle`
/// into a per-request scene route action, giving the handler full ECS access.
pub fn scene_system<Func, Input, B, FnMarker>(
	handler: Func,
) -> Action<Input, SceneEntity>
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = B>,
	Func: IntoSystem<
			In<ActionContext<Input>>,
			B,
			(IsFunctionSystem, FnMarker),
		>,
	Input: 'static + Send + Sync,
	B: 'static + Send + Sync + Bundle,
{
	Action::new_system(handler).chain(spawn_scene_step::<B>())
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use bevy::ecs::system::In;

	fn router_world() -> World { (AsyncPlugin, RouterPlugin).into_world() }

	#[beet_core::test]
	async fn renders_async_handler() {
		async fn home(_cx: ActionContext<Request>) -> impl Bundle {
			rsx! { <p>"async home"</p> }
		}
		router_world()
			.spawn((router(), children![exchange_route(
				"home",
				scene_async(home)
			)]))
			.call::<Request, Response>(Request::get("home"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("async home");
	}

	#[beet_core::test]
	async fn renders_system_handler() {
		fn home(_cx: In<ActionContext<Request>>) -> impl Bundle {
			rsx! { <p>"system home"</p> }
		}
		router_world()
			.spawn((router(), children![exchange_route(
				"home",
				scene_system(home)
			)]))
			.call::<Request, Response>(Request::get("home"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("system home");
	}

	#[beet_core::test]
	async fn rebuilds_per_request() {
		async fn home(_cx: ActionContext<Request>) -> impl Bundle {
			rsx! { <p>"home"</p> }
		}
		let mut world = router_world();
		let root = world
			.spawn((router(), children![exchange_route(
				"home",
				scene_async(home)
			)]))
			.flush();
		// two requests each succeed, proving per-request rebuild + cleanup
		for _ in 0..2 {
			world
				.entity_mut(root)
				.call::<Request, Response>(Request::get("home"))
				.await
				.unwrap()
				.unwrap_str()
				.await
				.xpect_contains("home");
		}
	}
}
