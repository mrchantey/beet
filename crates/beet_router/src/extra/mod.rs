//! App-level router features built on the core routing primitives.
//!
//! Unlike the generic middleware and dispatch in [`router`](crate::router),
//! these are opinionated, ready-made building blocks: package-info and
//! analytics routes, the [`HtmlStore`] prebuilt-HTML gate, and a
//! batteries-included [`default_router`].

mod html_store;
pub use html_store::*;
// the shared static-host serve rules (`serve_blob`) for a [`BlobStore`], used by
// `BlobStoreRoute` and the HTML-store gate (no_std core).
mod blob_store;
pub use blob_store::*;
// the standard blob-store agent toolset + a markup store mount, composing
// `exchange_route` with beet_net's blob-store actions.
#[cfg(feature = "std")]
mod store_toolset;
#[cfg(feature = "std")]
pub use store_toolset::*;

// std-only: the analytics route stores into beet_net's `AnalyticsEvent`, which
// is part of beet_net's std-only store surface.
#[cfg(all(feature = "json", feature = "std"))]
mod analytics;
#[cfg(all(feature = "json", feature = "std"))]
pub use analytics::*;

// std-only: the app-info scene route renders through beet_ui, and the
// batteries-included `default_router` wires it as one of its children when std.
#[cfg(feature = "std")]
mod app_info;
#[cfg(feature = "std")]
pub use app_info::*;

// std-only: the reactivity runtime route serves beet_ui's `REACTIVITY_JS`, the
// shared asset the renderer's auto-injected `<script defer>` loads.
#[cfg(feature = "std")]
mod reactivity_js;
#[cfg(feature = "std")]
pub use reactivity_js::*;

// std-only: the `/health` route (uptime + active sessions derived from world
// state), the load-balancer health check and autoscaling signal.
#[cfg(feature = "std")]
mod health;
#[cfg(feature = "std")]
pub use health::*;
// The single router builder, available on std and no_std. The feature-specific
// app routes (`app-info`, `analytics`) are gated inside the module.
mod default_router;
pub use default_router::*;
