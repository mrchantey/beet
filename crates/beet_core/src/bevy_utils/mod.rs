//! Bevy utility components, systems, and helpers.
//!
//! This module provides reusable utilities for working with Bevy, including
//! async command execution, entity management, and common system patterns.
//!
//! # Async Utilities
//!
//! - [`AsyncCommands`] - Execute commands from async contexts
//! - [`AsyncRunner`] - Run apps asynchronously to completion
//!
//! # Entity Utilities
//!
//! - [`AncestorQuery`] - Query entities through ancestor relationships
//! - [`EntityTargetEvent`] - Events targeting specific entities
//! - [`Maybe`] - Optional component query wrapper
//!
//! # Systems and Plugins
//!
//! - [`GarbageCollect`] - Automatic cleanup of marked entities
//! - [`OnSpawn`] - Run logic when entities are spawned
//! - [`When`] - Conditional system execution
//! - [`NonSendPlugin`] - Plugin trait for non-send resources
//!
//! # Debugging
//!
//! - [`PrettyTracing`] - Enhanced tracing output for Bevy apps
//! - [`IdCounter`] - Unique ID generation

mod ancestor_query;
mod async_commands;
mod async_runner;
mod common_systems;
mod entity_target_event;
mod garbage_collect;
mod id_counter;
mod maybe;
mod non_send_marker;
mod non_send_plugin;
pub mod observer_ext;
mod on_spawn;
mod pretty_tracing;
mod when;

pub use ancestor_query::*;
pub use async_commands::*;
pub use async_runner::*;
pub use common_systems::*;
pub use entity_target_event::*;
pub use garbage_collect::*;
pub use id_counter::*;
pub use maybe::*;
pub use non_send_marker::*;
pub use non_send_plugin::*;
pub use on_spawn::*;
pub use pretty_tracing::*;
pub use when::*;
