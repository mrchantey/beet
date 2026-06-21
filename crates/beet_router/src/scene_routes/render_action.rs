//! Route constructors — turn a path + handler into a complete route.
//!
//! **Page handlers return `impl Bundle`** (the authoring default; an `rsx!`
//! tree is one), served by the route family the codegen emits:
//!
//! - [`fixed_func_route`] — a static page from `fn() -> impl Bundle`.
//! - [`func_route`] — a content fn `fn(Props) -> impl Bundle` built via `Default`.
//! - [`pure_route`] — `fn(cx: ActionContext<In>) -> impl Bundle`.
//! - [`async_route`] — `async fn(cx: ActionContext<In>) -> impl Bundle`.
//! - [`system_route`] — `fn(cx: In<ActionContext<In>>, ..) -> impl Bundle`.
//!
//! The router dispatches actions whose output must be [`IntoResponseAsync`]. The
//! per-request constructors bridge this: a handler becomes an
//! `Action<In, PageRequest>` called on every request that runs the handler,
//! builds the result through the template substrate (`spawn_template`) as an
//! ephemeral render root, and hands back its [`PageRequest`]. The renderer
//! serializes it and despawns it via [`DespawnAfterRender`].
//!
//! Building through `spawn_template` (not a bare `world.spawn`) is what resolves
//! the content's slots and fires its `On<SpawnTemplate>`/`On<LoadTemplate>`
//! lifecycle, so a page composed of `#[template]` widgets renders correctly.
//!
//! For pages whose content depends on the request, `In` is typically [`Request`]
//! (the full request, the same shape the per-request scene-route actions use).

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::Snippet;
use bevy::ecs::system::In;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;

/// A per-request scene route from a sync handler
/// `fn(cx: ActionContext<In>) -> impl Bundle`.
pub fn pure_route<Func, Input, B, M1>(path: &str, handler: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> B,
	Input: 'static + Send + Sync + FromRequest<M1>,
	B: 'static + Send + Sync + Bundle,
{
	exchange_route(
		path,
		Action::new_pure(handler).chain(spawn_render_step::<B>()),
	)
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
/// bundle produced by a handler, builds it through the template substrate as an
/// ephemeral render root, and returns its [`PageRequest`]. The entity is
/// despawned after render.
///
/// Building via `spawn_template` (rather than a bare `world.spawn`) runs the
/// slot resolution and lifecycle for content composed of `#[template]` widgets.
fn spawn_render_step<B: 'static + Send + Sync + Bundle>()
-> Action<B, PageRequest> {
	Action::new_async(|cx: ActionContext<B>| async move {
		let (caller, bundle) = (cx.caller, cx.input);
		caller
			.world()
			.with(move |world: &mut World| -> Result<PageRequest> {
				let mut entity =
					world.spawn_template(Snippet::from_bundle(bundle))?;
				let id = entity.id();
				PageRoot::insert(&mut entity, vec![id]);
				Ok(PageRequest(id))
			})
			.await
	})
}

/// Shorthand for a route whose handler is a plain constructor that ignores the
/// request — a `#[template]`-built content function. Builds the content `Props`
/// via `Default` and calls the constructor each request.
///
/// ```no_run
/// # use beet_router::prelude::*;
/// # use beet_action::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_ui::prelude::*;
/// fn home() -> impl Bundle { Element::new("p").with_inner_text("Home") }
/// render_action::func_route("home", |_: ()| home());
/// ```
pub fn func_route<Func, Props, B>(path: &str, ctor: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn(Props) -> B,
	Props: 'static + Send + Sync + Default,
	B: 'static + Send + Sync + Bundle,
{
	pure_route(path, move |_cx: ActionContext<Request>| {
		ctor(Props::default())
	})
}

/// A static page from a no-argument handler `fn() -> impl Bundle`, the
/// page-codegen default for a `.rs` page. The handler runs fresh each request
/// (nothing is cached at serve time); static export is governed separately by
/// [`ExportStrategy`].
pub fn fixed_func_route<Func, B>(path: &str, handler: Func) -> impl Bundle
where
	Func: 'static + Send + Sync + Clone + Fn() -> B,
	B: 'static + Send + Sync + Bundle,
{
	pure_route(path, move |_cx: ActionContext<Request>| handler())
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
			.spawn((default_router(), children![render_action::async_route(
				"home", home
			)]))
			.exchange(Request::get("home"))
			.await
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
			.spawn((default_router(), children![render_action::system_route(
				"home", home
			)]))
			.exchange(Request::get("home"))
			.await
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
			.spawn((default_router(), children![render_action::async_route(
				"home", home
			)]))
			.flush();
		// two requests each succeed, proving per-request rebuild + cleanup
		for _ in 0..2 {
			world
				.entity_mut(root)
				.exchange(Request::get("home"))
				.await
				.unwrap_str()
				.await
				.xpect_contains("home");
		}
	}
}
