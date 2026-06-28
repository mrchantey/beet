// no_std core: route tree, path patterns, standalone middleware, and the
// server-action client.
mod cors;
pub use cors::*;
mod no_cache;
pub use no_cache::*;
mod exchange_overload;
pub use exchange_overload::*;
mod exchange_fallback;
pub use exchange_fallback::*;
mod exchange_sequence;
pub use exchange_sequence::*;
mod behavior_sequence;
pub use behavior_sequence::*;
// the typed `ExchangeOverloadScript` route marker, the `ScriptRoute` front-end,
// and the `ExchangeScriptElement` console-capturing `<script>` entry action.
#[cfg(feature = "scripting")]
mod exchange_script;
#[cfg(feature = "scripting")]
pub use exchange_script::*;
// the `<Template src>` include: needs the BSX tag seam + the unified loader. It
// reads through the store as an async pending dependency, so it relies on the
// async runtime that `bsx` (→ `std`) pulls in (the same one `RoutesDir` uses).
#[cfg(all(feature = "bsx", feature = "template_serde"))]
mod template_include;
#[cfg(all(feature = "bsx", feature = "template_serde"))]
pub use template_include::*;
mod request_logger;
pub use request_logger::*;
mod interrupt;
pub use interrupt::*;
// std-only: `ArticleMeta::sidebar` is a `SidebarInfo`, which lives in the
// std-only `sidebar` module.
#[cfg(feature = "std")]
mod article_meta;
#[cfg(feature = "std")]
pub use article_meta::*;
mod request_context;
pub use request_context::*;
mod middleware;
pub use middleware::*;
mod route_tree;
pub use route_tree::*;
mod server_action_client;
pub use server_action_client::*;

// The `Router` dispatch action and the route-building `RouterPlugin` are shared
// across std and no_std (one `Router` type, one plugin). The single builder that
// assembles them with the standard middleware and app routes is `default_router`
// (in `extra`). The std-only scene/help rendering pipeline stays feature-gated
// inside these and in the `help`/`sidebar` modules below; the no_std build falls
// back to a plain-text route listing.
mod router;
pub use router::*;
mod router_plugin;
pub use router_plugin::*;

// std-only: the help/sidebar rendering built on the beet_ui scene pipeline.
#[cfg(feature = "std")]
mod help;
#[cfg(feature = "std")]
pub use help::*;
#[cfg(feature = "std")]
mod base_layout;
#[cfg(feature = "std")]
pub use base_layout::*;
#[cfg(feature = "std")]
mod bsx_layout;
#[cfg(feature = "std")]
pub use bsx_layout::*;
#[cfg(feature = "std")]
mod sidebar;
#[cfg(feature = "std")]
pub use sidebar::*;
#[cfg(feature = "std")]
mod site_layout;
#[cfg(feature = "std")]
pub use site_layout::*;
// the browser-wasm page templates `<Wasm>` + `<MainBsx>`: serve-side, building a
// page that boots a wasm `beet` binary and references its `.bsx` program. Plain
// synchronous templates, so they render inside a route's content (std-gated like
// the rest of the render pipeline).
#[cfg(feature = "std")]
mod wasm;
#[cfg(feature = "std")]
pub use wasm::*;
