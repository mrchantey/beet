// no_std core: route tree, path patterns, standalone middleware, and the
// server-action client.
mod cors;
pub use cors::*;
mod no_cache;
pub use no_cache::*;
mod exchange_action;
pub use exchange_action::*;
mod exchange_fallback;
pub use exchange_fallback::*;
mod exchange_sequence;
pub use exchange_sequence::*;
#[cfg(feature = "scripting")]
mod exchange_script;
#[cfg(feature = "scripting")]
pub use exchange_script::*;
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
mod middleware;
pub use middleware::*;
mod route_tree;
pub use route_tree::*;
mod server_action_client;
pub use server_action_client::*;

// The `Router` dispatch action, the `router()` bundle, and the route-building
// `RouterPlugin` are shared across std and no_std (one `Router` type, one
// plugin). The std-only scene/help rendering pipeline stays feature-gated inside
// them and in the `help`/`sidebar` modules below; the no_std build falls back to
// a plain-text route listing and a lean `router()` bundle.
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
mod sidebar;
#[cfg(feature = "std")]
pub use sidebar::*;
