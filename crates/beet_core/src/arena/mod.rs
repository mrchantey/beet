//! Global arenas for storing objects with copyable handles.
//!
//! This module provides arena-based storage where objects are stored in a global
//! collection and accessed through lightweight, copyable handles. This pattern
//! is useful when you need to pass references around without lifetime constraints.
//!
//! # Types
//!
//! - [`Arena`] / [`ArenaHandle`] - Thread-safe arena for `Send` types
//! - [`Store`] - Ergonomic wrapper around `ArenaHandle` with value semantics
//!
//! # Warning
//!
//! Arena handles must be manually removed when no longer needed in long-running
//! applications. In short-lived contexts like tests, leaking is usually harmless.

mod arena;
mod store;

pub(crate) use arena::*;
pub use store::*;
