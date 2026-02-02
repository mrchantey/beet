//! General utilities and extension traits.
//!
//! This module provides a collection of utilities used throughout the beet
//! framework, including async helpers, method chaining, and cross-platform
//! abstractions.
//!
//! # Key Types
//!
//! - [`Xtend`] - Method chaining utilities for any type
//! - [`BevyhowError`] - Error type for use with Bevy's error handling
//! - [`Tree`] - Simple tree data structure
//! - [`GlobFilter`] - Glob pattern matching utilities
//!
//! # Macros
//!
//! - [`bevyhow!`](crate::bevyhow) - Create a [`BevyError`](bevy::ecs::error::BevyError) with formatting
//! - [`bevybail!`](crate::bevybail) - Early return with a [`BevyError`](bevy::ecs::error::BevyError)
//! - [`cross_log!`](crate::cross_log) - Cross-platform logging (works in wasm)

/// Async utilities and future helpers.
pub mod async_ext;
mod backoff;
mod bevyhow;
mod cli_args;
mod clone_func;
mod cross_log;
/// Display formatting utilities.
pub mod display_ext;
mod file_span;
mod glob_filter;
mod line_col;
#[cfg(feature = "ansi_paint")]
pub mod paint_ext;
mod panic_context;
/// Process and command execution utilities.
pub mod process_ext;
#[cfg(feature = "rand")]
mod random_source;
pub mod terminal_ext;
/// Time and duration utilities.
pub mod time_ext;
mod tree;
#[cfg(feature = "serde")]
pub mod type_info_to_json_schema;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
mod xtend;

pub use async_ext::LifetimeSendBoxedFuture;
pub use async_ext::MaybeSendBoxedFuture;
pub use async_ext::SendBoxedFuture;
pub use backoff::*;
pub use bevyhow::*;
pub use cli_args::*;
pub use clone_func::*;
pub use file_span::*;
pub use glob_filter::*;
pub use line_col::*;
pub use panic_context::*;
#[cfg(feature = "rand")]
pub use random_source::*;
pub use tree::*;
pub use xtend::*;
