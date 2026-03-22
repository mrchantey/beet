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
//! - [`GlobFilter`] - Glob pattern matching utilities (requires `std`)
//! # Macros
//!
//! - [`cross_log!`](crate::cross_log) - Cross-platform logging (works in wasm)

mod as_any;
/// Async utilities and future helpers.
#[cfg(feature = "std")]
pub mod async_ext;
mod backoff;
#[cfg(feature = "std")]
mod cli_args;
mod clone_func;
pub mod cross_log;
/// Display formatting utilities.
pub mod display_ext;
#[cfg(feature = "std")]
mod file_span;
#[cfg(feature = "std")]
mod glob_filter;
mod line_col;
#[cfg(feature = "ansi_paint")]
pub mod paint_ext;
#[cfg(feature = "std")]
mod panic_context;
/// Process and command execution utilities.
#[cfg(feature = "std")]
pub mod process_ext;
#[cfg(feature = "rand")]
mod random_source;
/// Stream conversion utilities for byte-to-text streaming.
#[cfg(feature = "std")]
pub mod stream_ext;
#[cfg(feature = "std")]
pub use stream_ext::TextStream;
#[cfg(feature = "std")]
pub mod terminal_ext;
/// Time and duration utilities.
#[cfg(feature = "std")]
pub mod time_ext;
mod tree;
/// Timer utilities for WebAssembly environments.
#[cfg(target_arch = "wasm32")]
pub mod wasm;
mod xtend;
pub use as_any::*;

#[cfg(feature = "std")]
pub use async_ext::LifetimeSendBoxedFuture;
#[cfg(feature = "std")]
pub use async_ext::MaybeSendBoxedFuture;
#[cfg(feature = "std")]
pub use async_ext::SendBoxedFuture;
pub use backoff::*;
pub use bevy::tasks::BoxedFuture;
#[cfg(feature = "std")]
pub use cli_args::*;
pub use clone_func::*;
#[cfg(feature = "std")]
pub use file_span::*;
#[cfg(feature = "std")]
pub use glob_filter::*;
pub use line_col::*;
#[cfg(feature = "std")]
pub use panic_context::*;
#[cfg(feature = "rand")]
pub use random_source::*;
pub use tree::*;
pub use xtend::*;
