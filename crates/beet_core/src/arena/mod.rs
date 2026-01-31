//! Global arenas for storing objects with copyable handles.
//!
//! This module provides arena-based storage where objects are stored in a global
//! collection and accessed through lightweight, copyable handles. This pattern
//! is useful when you need to pass references around without lifetime constraints.
//!
//! # Types
//!
//! - [`Arena`] / [`ArenaHandle`] - Thread-safe arena for `Send` types
//! - [`NonSendArena`] / [`NonSendArenaHandle`] - Thread-local arena for non-`Send` types
//! - [`Store`] - Ergonomic wrapper around `ArenaHandle` with value semantics
//! - [`Signal`] / [`Getter`] / [`Setter`] - Simple reactive primitives
//! - [`FuncStore`] - Function wrapper that records call outputs
//!
//! # Warning
//!
//! Arena handles must be manually removed when no longer needed in long-running
//! applications. In short-lived contexts like tests, leaking is usually harmless.

mod arena;
mod func_store;
mod non_send_arena;
mod signal;
mod store;

pub use arena::*;
pub use func_store::*;
pub use non_send_arena::*;
pub use signal::*;
pub use store::*;
