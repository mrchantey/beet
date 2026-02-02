//! General parsing utilities for both the beet cli and various macros.
//!
//! This crate provides utilities for:
//!
//! - [`derive`]: Derive macro utilities and token generation
//! - [`lang`]: Language parsing primitives
//! - [`parse_rsx_tokens`]: RSX template parsing
//! - [`tokenize`]: Tokenization utilities
//! - [`utils`]: Common parsing helpers
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![feature(if_let_guard, exact_size_is_empty)]
#![deny(missing_docs)]

/// Derive macro utilities and token generation helpers.
pub mod derive;
/// Language parsing primitives including syntax highlighting.
pub mod lang;
/// RSX template token parsing and element collection.
pub mod parse_rsx_tokens;
/// Tokenization utilities for structs, templates, and event handlers.
pub mod tokenize;
/// Common parsing helper functions and types.
pub mod utils;

/// Re-exports of commonly used parsing utilities.
pub mod prelude {
	pub use crate::derive::*;
	#[allow(unused)]
	pub use crate::lang::*;
	pub use crate::parse_rsx_tokens::*;
	pub use crate::tokenize::*;
	pub use crate::utils::*;
}

/// Re-exports of external crates used by beet_parse.
pub mod exports {
	pub use send_wrapper::SendWrapper;
}
