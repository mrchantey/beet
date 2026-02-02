//! Shared utilities for `beet_core` and `beet_core_macros`.
//!
//! This crate contains token utilities that need to be compiled for both
//! the main `beet_core` crate and the `beet_core_macros` proc-macro crate.

mod attribute_group;
mod named_field;
/// Package configuration extensions for Cargo.toml parsing.
pub mod pkg_ext;

pub use attribute_group::*;
pub use named_field::*;
