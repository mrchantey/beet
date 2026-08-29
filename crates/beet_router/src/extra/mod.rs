//! App-level router features built on the core routing primitives.
//!
//! Unlike the generic middleware and dispatch in [`router`](crate::router),
//! these are opinionated, ready-made building blocks: package-info and
//! analytics routes, and a batteries-included [`Router::with_defaults`].

// the shared static-host serve rules (`serve_blob`) for a [`BlobStore`], used by
// `ServeBlobs` (no_std core).
mod blob_store;
pub(crate) use blob_store::*;
// the standard blob-store agent toolset + a markup store mount, composing
// `exchange_route` with beet_net's blob-store actions.
#[cfg(feature = "std")]
mod store_toolset;
#[cfg(feature = "std")]
pub use store_toolset::*;

// std: the analytics emitters build beet_net's `AnalyticsEvent` (serde, via std);
// only the beacon route's json-body parsing needs `json`, gated inside.
#[cfg(feature = "std")]
mod analytics;
#[cfg(feature = "std")]
pub use analytics::*;

// std-only: the app-info scene route renders through beet_ui, and the
// batteries-included `Router::with_defaults` wires it as one of its children when std.
#[cfg(feature = "std")]
mod app_info;
#[cfg(feature = "std")]
pub(crate) use app_info::*;

// std-only: the reactivity runtime route serves beet_ui's `Reactivity::JS`, the
// shared asset the renderer's auto-injected `<script defer>` loads.
#[cfg(feature = "std")]
mod reactivity_js;
#[cfg(feature = "std")]
pub(crate) use reactivity_js::*;

// std-only: the `/health` route (uptime + active sessions derived from world
// state), the load-balancer health check and autoscaling signal.
#[cfg(feature = "std")]
mod health;
#[cfg(feature = "std")]
pub(crate) use health::*;
// The single router builder, available on std and no_std. The feature-specific
// app routes (`app-info`, `analytics`) are gated inside the module.
mod default_router;
pub use default_router::*;
