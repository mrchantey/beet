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
// reactive template registration: a `<TemplateDir src="templates"/>` reads its
// dir through the nearest ancestor [`BlobStore`] and registers each template,
// resolved by `AncestorQuery<&BlobStore>` like `RoutesDir`.
#[cfg(feature = "bsx")]
mod template_dir;
#[cfg(feature = "bsx")]
pub use template_dir::*;
// the entry-declared store root (`<StoreRoot src="../.."/>`), pre-scanned by
// entry resolution like an entry's own `<TemplateDir>`s.
#[cfg(feature = "bsx")]
mod store_root;
#[cfg(feature = "bsx")]
pub use store_root::*;
// `RoutesDir` + its discovery is compiled on every std target: one observer
// scans the store off the async runtime, so native and wasm share the path.
mod routes_dir;
pub use routes_dir::*;
// the `ServeBlobs` static-file route: it owns its mount prefix and serves from the
// nearest `BlobStore`. cross-platform: the wasm Worker serves a site's assets too.
mod serve_blobs;
pub use serve_blobs::*;
