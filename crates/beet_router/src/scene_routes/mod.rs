//! Scene routes — routes that return a renderable entity tree (a scene) rather
//! than serialized data.
//!
//! A regular `exchange_route` returns an `ExchangeRouteOut` (JSON, a redirect,
//! bytes) already in final form. A *scene route* instead yields the [`Entity`]
//! root of a tree (an rsx/markdown/parsed document, a behavior tree, …)
//! carrying a [`PageRoot`]; a `NodeRenderer` walks [`PageRoot::rendered`]
//! and serializes it per the request's `Accept` header (HTML, markdown,
//! charcell, …), then despawns the [`DespawnAfterRender`] entities.
//!
//! Handlers produce a content [`Bundle`]; the [`render_action`] constructors
//! build a complete route from a path + handler:
//! [`render_action::fixed_func_route`] (static, per request), and
//! [`render_action::pure_route`] / [`render_action::async_route`] /
//! [`render_action::system_route`] (per handler kind). The tree is serialized
//! by [`default_renderer`].

mod page_root;
pub use page_root::*;
mod default_renderer;
pub mod render_action;
pub use default_renderer::*;
mod route_query;
pub use route_query::*;
// the site root (a path) backs both `RoutesDir` and `<Template src>` includes, so
// it is always compiled, unlike the native-only directory-scan modules below.
mod site_root;
pub use site_root::*;
#[cfg(not(target_arch = "wasm32"))]
mod routes_dir;
#[cfg(not(target_arch = "wasm32"))]
pub use routes_dir::*;
#[cfg(not(target_arch = "wasm32"))]
mod blob_store_route;
#[cfg(not(target_arch = "wasm32"))]
pub use blob_store_route::*;
