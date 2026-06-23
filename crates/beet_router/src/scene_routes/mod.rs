//! Scene routes — routes that return a renderable entity tree (a scene) rather
//! than serialized data.
//!
//! A regular `exchange_route` returns an `IntoResponseAsync` (JSON, a redirect,
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
// reading a site's `templates/` through its [`BlobStore`]; the store itself is
// composed on the loaded root entity and resolved by `AncestorQuery<&BlobStore>`.
#[cfg(feature = "bsx")]
mod site_templates;
#[cfg(feature = "bsx")]
pub use site_templates::*;
// `RoutesDir` + its discovery is compiled on every std target: one observer
// scans the store off the async runtime, so native and wasm share the path.
mod routes_dir;
pub use routes_dir::*;
// the static-asset mount is native-only (no wasm consumer needs it yet).
#[cfg(not(target_arch = "wasm32"))]
mod blob_store_route;
#[cfg(not(target_arch = "wasm32"))]
pub use blob_store_route::*;
