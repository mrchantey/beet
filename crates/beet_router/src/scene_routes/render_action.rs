//! Scene-route constructors — turn a path + handler into a complete route.
//!
//! The four public constructors are the intended downstream surface:
//!
//! - [`fixed_route`] — `fn() -> impl Bundle`, a static page spawned once.
//! - [`pure_route`] — `fn(cx: ActionContext<In>) -> impl Bundle`.
//! - [`async_route`] — `async fn(cx: ActionContext<In>) -> impl Bundle`.
//! - [`system_route`] — `fn(cx: In<ActionContext<In>>, ..system params) -> impl Bundle`.
//!
//! File-based page handlers return content as an `impl Bundle`, but the router
//! dispatches actions whose output must be [`ExchangeRouteOut`]. The per-request
//! constructors bridge the two: a handler becomes an `Action<In, RenderRequest>`
//! called on every request that runs the handler, spawns the bundle as an
//! ephemeral render root, and hands back its [`RenderRequest`]. The renderer
//! then serializes it and despawns it via [`DespawnAfterRender`].
//!
//! For pages whose content depends on the request, `In` is typically [`Request`]
//! (the full request, the same shape route actions like `CallerScene` use).

use super::render_root::CallerScene;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::ecs::system::In;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;

/// A static page spawned once: the entity becomes both the route and the
/// rendered content, marked a render root by [`CallerScene`].
///
/// # Example
///
/// ```no_run
/// use beet_router::prelude::*;
/// use beet_core::prelude::*;
/// use beet_ui::prelude::*;
///
/// let bundle = render_action::fixed_route("about",
///     Element::new("p").with_inner_text("About page")
/// );
/// ```
pub fn fixed_route<B: Bundle>(path: &str, bundle: B) -> impl Bundle {
	route(path, (CallerScene, bundle))
}

/// A per-request scene route from a sync handler
/// `fn(cx: ActionContext<In>) -> impl Bundle`.
pub fn pure_route<Func, Input, B, M1>(path: &str, handler: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> B,
	Input: 'static + Send + Sync + FromRequest<M1>,
	B: 'static + Send + Sync + Bundle,
{
	exchange_route(path, Action::new_pure(handler).chain(spawn_render_step::<B>()))
}

/// A per-request scene route from an async handler
/// `async fn(cx: ActionContext<In>) -> impl Bundle`.
pub fn async_route<Func, Input, Fut, B, M1>(
	path: &str,
	handler: Func,
) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> Fut,
	Fut: 'static + MaybeSend + Future<Output = B>,
	Input: 'static + Send + Sync + FromRequest<M1>,
	B: 'static + Send + Sync + Bundle,
{
	exchange_route(
		path,
		Action::new_async(handler).chain(spawn_render_step::<B>()),
	)
}

/// A per-request scene route from a system handler
/// `fn(cx: In<ActionContext<In>>, ..system params) -> impl Bundle`.
pub fn system_route<Func, Input, B, FnMarker, M1>(
	path: &str,
	handler: Func,
) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = B>,
	Func: IntoSystem<In<ActionContext<Input>>, B, (IsFunctionSystem, FnMarker)>,
	Input: 'static + Send + Sync + FromRequest<M1>,
	B: 'static + Send + Sync + Bundle,
{
	exchange_route(
		path,
		Action::new_system(handler).chain(spawn_render_step::<B>()),
	)
}

/// The shared second half of every per-request route constructor: takes the
/// bundle produced by a handler, spawns it as an ephemeral render root, and
/// returns its [`RenderRequest`]. The entity is despawned after render.
fn spawn_render_step<B: 'static + Send + Sync + Bundle>()
-> Action<B, RenderRequest> {
	Action::new_async(|cx: ActionContext<B>| async move {
		let (caller, bundle) = (cx.caller, cx.input);
		caller
			.world()
			.with_then(move |world: &mut World| {
				let mut entity = world.spawn(bundle);
				let id = entity.id();
				RenderRoot::insert(&mut entity, vec![id]);
				RenderRequest(id)
			})
			.await
			.xok()
	})
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
			.spawn((router(), children![render_action::async_route(
				"home", home
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
			.spawn((router(), children![render_action::system_route(
				"home", home
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
			.spawn((router(), children![render_action::async_route(
				"home", home
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
