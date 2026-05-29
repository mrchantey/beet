//! App-level router features built on the core routing primitives.
//!
//! Unlike the generic middleware and dispatch in [`router`](crate::router),
//! these are opinionated, ready-made building blocks: package-info and
//! analytics routes, the [`HtmlStore`] prebuilt-HTML gate, and a
//! batteries-included [`default_router`].

mod html_store;
pub use html_store::*;
// serving static files from a [`BlobStore`] as routes (no_std core).
mod blob_store;
pub use blob_store::*;

#[cfg(feature = "json")]
mod analytics;
#[cfg(feature = "json")]
pub use analytics::*;

// std-only: the app-info scene route renders through beet_ui, and the
// batteries-included `default_router` wires it alongside `router()`.
#[cfg(feature = "std")]
mod app_info;
#[cfg(feature = "std")]
pub use app_info::*;
#[cfg(all(feature = "json", feature = "std"))]
mod default_router;
#[cfg(all(feature = "json", feature = "std"))]
pub use default_router::*;
