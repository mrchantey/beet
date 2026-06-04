//! Route constructors — turn a path + handler into a complete route.
//!
//! **Page handlers return `impl Scene`** (the authoring default), served by the
//! scene-route family the codegen emits:
//!
//! - [`fixed_scene_route`] — `fn() -> impl Scene`, a static page.
//! - [`scene_route`] — `fn(cx: ActionContext<In>) -> impl Scene`.
//! - [`async_scene_route`] — `async fn(cx: ActionContext<In>) -> impl Scene`.
//! - [`system_scene_route`] — `fn(cx: In<ActionContext<In>>, ..) -> impl Scene`.
//!
//! The `impl Bundle` family ([`fixed_route`]/[`pure_route`]/[`async_route`]/
//! [`system_route`]) is the lower-level primitive for content built without the
//! scene layer (eg `rsx_direct!`); it is no longer the page-authoring default.
//!
//! Either way the router dispatches actions whose output must be
//! [`ExchangeRouteOut`]. The per-request constructors bridge this: a handler
//! becomes an `Action<In, RenderRequest>` called on every request that runs the
//! handler, spawns the result as an ephemeral render root, and hands back its
//! [`RenderRequest`]. The renderer then serializes it and despawns it via
//! [`DespawnAfterRender`].
//!
//! For pages whose content depends on the request, `In` is typically [`Request`]
//! (the full request, the same shape route actions like `CallerScene` use).

use super::render_root::CallerScene;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::Scene;
use beet_ui::prelude::WorldSceneExt;
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
			.with(move |world: &mut World| {
				let mut entity = world.spawn(bundle);
				let id = entity.id();
				RenderRoot::insert(&mut entity, vec![id]);
				RenderRequest(id)
			})
			.await
			.xok()
	})
}

/// A per-request scene route from a sync handler returning an [`impl Scene`].
///
/// Like [`pure_route`] but uses Bevy's scene system instead of bundle spawning:
/// the handler returns an `impl Scene` (typically from a `#[scene]` /
/// `#[scene(system)]` function or `rsx!`), which is resolved and spawned
/// per request as an ephemeral render root.
pub fn scene_route<Func, Input, S, M1>(path: &str, handler: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> S,
	Input: 'static + Send + Sync + FromRequest<M1>,
	S: 'static + Send + Sync + Scene,
{
	exchange_route(path, Action::new_pure(handler).chain(spawn_scene_step::<S>()))
}

/// Shorthand for [`scene_route`] when the handler is a plain constructor that
/// ignores the request — `<Foo as SceneComponent>::scene`-style. Builds the
/// props via `Default` and calls the constructor each request.
///
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_action::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_ui::prelude::SceneComponent;
/// # #[derive(Default)] struct AppInfo;
/// # impl AppInfo {
/// #     fn scene(_: ()) -> impl beet_ui::prelude::Scene { () }
/// # }
/// // closure form (annotate the input so `scene_route` can pick a `FromRequest`)
/// render_action::scene_route(
///     "app-info",
///     |_: ActionContext<Request>| AppInfo::scene(()),
/// );
/// // shorthand
/// render_action::scene_func_route("app-info", AppInfo::scene);
/// ```
pub fn scene_func_route<Func, Props, S>(
	path: &str,
	ctor: Func,
) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(Props) -> S,
	Props: 'static + Send + Sync + Default,
	S: 'static + Send + Sync + Scene,
{
	scene_route(path, move |_cx: ActionContext<Request>| ctor(Props::default()))
}

/// A static scene page from a no-argument handler `fn() -> impl Scene`, the
/// scene equivalent of [`fixed_route`]. The handler is called per request (the
/// route is typically marked `CacheStrategy::Static`, so the result is cached).
pub fn fixed_scene_route<Func, S>(path: &str, handler: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn() -> S,
	S: 'static + Send + Sync + Scene,
{
	scene_route(path, move |_cx: ActionContext<Request>| handler())
}

/// A per-request scene route from an async handler
/// `async fn(cx: ActionContext<In>) -> impl Scene`, the scene equivalent of
/// [`async_route`].
pub fn async_scene_route<Func, Input, Fut, S, M1>(
	path: &str,
	handler: Func,
) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> Fut,
	Fut: 'static + MaybeSend + Future<Output = S>,
	Input: 'static + Send + Sync + FromRequest<M1>,
	S: 'static + Send + Sync + Scene,
{
	exchange_route(
		path,
		Action::new_async(handler).chain(spawn_scene_step::<S>()),
	)
}

/// A per-request scene route from a system handler
/// `fn(cx: In<ActionContext<In>>, ..system params) -> impl Scene`, the scene
/// equivalent of [`system_route`].
pub fn system_scene_route<Func, Input, S, FnMarker, M1>(
	path: &str,
	handler: Func,
) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = S>,
	Func: IntoSystem<In<ActionContext<Input>>, S, (IsFunctionSystem, FnMarker)>,
	Input: 'static + Send + Sync + FromRequest<M1>,
	S: 'static + Send + Sync + Scene,
{
	exchange_route(
		path,
		Action::new_system(handler).chain(spawn_scene_step::<S>()),
	)
}

/// The scene equivalent of [`spawn_render_step`]: resolves and spawns the
/// scene as an ephemeral render root.
fn spawn_scene_step<S: 'static + Send + Sync + Scene>()
-> Action<S, RenderRequest> {
	Action::new_async(|cx: ActionContext<S>| async move {
		let (caller, scene) = (cx.caller, cx.input);
		caller
			.world()
			.with(move |world: &mut World| -> Result<RenderRequest> {
				let mut entity = world.spawn_scene(scene)?;
				let id = entity.id();
				RenderRoot::insert(&mut entity, vec![id]);
				RenderRequest(id).xok()
			})
			.await
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
			rsx_direct!{ <p>"async home"</p> }
		}
		router_world()
			.spawn((default_router(), children![render_action::async_route(
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
			rsx_direct!{ <p>"system home"</p> }
		}
		router_world()
			.spawn((default_router(), children![render_action::system_route(
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
			rsx_direct!{ <p>"home"</p> }
		}
		let mut world = router_world();
		let root = world
			.spawn((default_router(), children![render_action::async_route(
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
