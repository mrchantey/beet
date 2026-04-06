//! Proc-macro token utilities for code generation.
//!
//! This module provides utilities for working with proc-macro tokens,
//! including tokenization of Rust types and span manipulation.
//!
//! # Features
//!
//! - [`TokenizeSelf`] - Trait for types that can convert themselves to tokens
//! - [`TokenizeComponents`] - Utilities for tokenizing ECS components
//! - [`Unspan`] - Remove span information from token streams
//!
//! # Platform Support
//!
//! This module requires the `tokens` feature flag.

pub use beet_core_shared::prelude::*;
mod tokenize_components;
mod tokenize_self;
mod unspan;
pub use tokenize_components::*;
pub use tokenize_self::*;
pub use unspan::*;
