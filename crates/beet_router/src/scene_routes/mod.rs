//! Scene routes — routes that return a renderable entity tree (a scene) rather
//! than serialized data.
//!
//! A regular `exchange_route` returns an `ExchangeRouteOut` (JSON, a redirect,
//! bytes) already in final form. A *scene route* instead yields the [`Entity`]
//! root of a tree (an rsx/markdown/parsed document, a behavior tree, …)
//! carrying a [`RenderRoot`]; a `NodeRenderer` walks [`RenderRoot::rendered`]
//! and serializes it per the request's `Accept` header (HTML, markdown,
//! charcell, …), then despawns the [`DespawnAfterRender`] entities.
//!
//! Handlers produce a content [`Bundle`]; the [`render_action`] constructors
//! build a complete route from a path + handler:
//! [`render_action::fixed_route`] (static, spawned once), and
//! [`render_action::pure_route`] / [`render_action::async_route`] /
//! [`render_action::system_route`] (per handler kind). The tree is serialized
//! by [`default_renderer`].

mod render_root;
pub use render_root::*;
pub mod render_action;
mod default_renderer;
pub use default_renderer::*;
mod route_query;
pub use route_query::*;
