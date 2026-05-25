//! App-level router features built on the core routing primitives.
//!
//! Unlike the generic middleware and dispatch in [`router`](crate::router),
//! these are opinionated, ready-made building blocks: package-info and
//! analytics routes, static-asset buckets, the [`RenderMode`] gate, and a
//! batteries-included [`default_router`].

mod app_info;
pub use app_info::*;
mod buckets;
pub use buckets::*;
mod render_mode;
pub use render_mode::*;

#[cfg(feature = "json")]
mod analytics;
#[cfg(feature = "json")]
pub use analytics::*;
#[cfg(feature = "json")]
mod default_router;
#[cfg(feature = "json")]
pub use default_router::*;
