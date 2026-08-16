//! General utilities and extension traits.
//!
//! This module provides a collection of utilities used throughout the beet
//! framework, including async helpers, method chaining, and cross-platform
//! abstractions.
//!
//! # Key Types
//!
//! - [`Xtend`] - Method chaining utilities for any type
//! - [`Tree`] - Simple tree data structure
//! - [`GlobFilter`] - Glob pattern matching utilities
//! # Macros
//!
//! - [`cross_log!`](crate::cross_log) - Cross-platform raw output (not for logging, see the macro docs)

mod as_any;
/// Async utilities and future helpers.
pub mod async_ext;
mod backoff;
mod cfg_if;
mod cli_args;
/// Coalescing trigger for async write deduplication.
mod coalescing_trigger;
pub mod cross_log;
/// Display formatting utilities.
pub mod display_ext;
// Cross-platform env access; no_std reads return "not found" so callers fall
// back to defaults (it does not need std like the rest of `path_utils`).
pub mod env_ext;
mod file_span;
mod glob_filter;
mod into_option;
// LazyPool is built on async_lock (std-only).
#[cfg(feature = "std")]
mod lazy_pool;
mod line_col;
/// A no_std one-shot value channel.
mod once_value;
#[cfg(any(feature = "std", feature = "testing_embedded"))]
mod panic_context;
/// Poll a fallible function until it succeeds or a deadline expires.
#[cfg(feature = "std")]
pub mod poll_ext;
/// Process and command execution utilities.
#[cfg(feature = "std")]
pub mod process_ext;
#[cfg(feature = "rand")]
mod random_source;
#[cfg(feature = "serde")]
pub mod serde_ext;
/// Stream conversion utilities for byte-to-text streaming.
#[cfg(feature = "std")]
pub mod stream_ext;
pub use into_option::*;
#[cfg(feature = "std")]
pub use stream_ext::TextStream;
/// Time and duration utilities. Sleep/clock helpers are std-gated per-function;
/// [`time_ext::pretty_print_duration`] works on no_std.
pub mod time_ext;
/// An absolute, serializable wall-clock instant, the persistable counterpart of
/// the monotonic [`Instant`].
mod timestamp;
/// Cross-platform uuid creation ([`uuid_ext::now_v7`]); `uuid`'s own v7 clock is
/// std-only, so this derives its timestamp from [`time_ext`] instead.
#[cfg(feature = "uuid")]
pub mod uuid_ext;
/// Typed physical quantities ([`units::Angle`], [`units::Distance`],
/// [`units::LinearVelocity`], [`units::AngularVelocity`]) shared by the robot
/// transport and the `SetDrive` action.
pub mod units;
#[cfg(feature = "std")]
pub use lazy_pool::*;
pub use units::*;
mod tree;
mod xtend;
pub use as_any::*;

pub use async_ext::LifetimeSendBoxedFuture;
pub use async_ext::LocalBoxedFuture;
pub use async_ext::MaybeSendBoxedFuture;
pub use async_ext::SendBoxedFuture;
pub use async_ext::SendSyncBoxedFuture;
pub use backoff::*;
pub use bevy::tasks::BoxedFuture;
pub use cli_args::*;
pub use coalescing_trigger::*;
pub use file_span::*;
pub use glob_filter::*;
pub use line_col::*;
pub use once_value::*;
#[cfg(any(feature = "std", feature = "testing_embedded"))]
pub(crate) use panic_context::*;
#[cfg(feature = "rand")]
pub use random_source::*;
pub use timestamp::*;
pub use tree::*;
pub use xtend::*;
