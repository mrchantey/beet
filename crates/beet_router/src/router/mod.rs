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

// no_std-only: a lightweight `router()` + dispatch + route-building plugin for
// bare-metal targets, without the std scene/help rendering pipeline. Compiled
// only when `std` is off, so its `Router`/`router()`/`RouterPlugin` never
// collide with the std versions below.
#[cfg(not(feature = "std"))]
mod embedded;
#[cfg(not(feature = "std"))]
pub use embedded::*;

// std-only: the assembled `router()` + dispatch, the route-building plugin,
// and the help/sidebar rendering — all built on the beet_ui scene pipeline.
#[cfg(feature = "std")]
mod help;
#[cfg(feature = "std")]
pub use help::*;
#[cfg(feature = "std")]
mod router;
#[cfg(feature = "std")]
pub use router::*;
#[cfg(feature = "std")]
mod router_plugin;
#[cfg(feature = "std")]
pub use router_plugin::*;
#[cfg(feature = "std")]
mod sidebar;
#[cfg(feature = "std")]
pub use sidebar::*;
