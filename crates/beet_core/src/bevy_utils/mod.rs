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
//! - [`BevyhowError`] - Error type for use with Bevy's error handling
//! - [`LogPlugin`] - Drop-in replacement for bevy's `LogPlugin` using [`PrettyTracing`]
//! - [`PrettyTracing`] - Enhanced tracing output for Bevy apps
//! - [`IdCounter`] - Unique ID generation
//!
//! # Macros
//!
//! - [`bevyhow!`](crate::bevyhow) - Create a [`BevyError`](bevy::ecs::error::BevyError) with formatting
//! - [`bevybail!`](crate::bevybail) - Early return with a [`BevyError`](bevy::ecs::error::BevyError)

mod ancestor_query;
// niche [`App`] free functions, kept off the `BeetCoreAppExt` trait
#[cfg(all(feature = "bevy_async", feature = "std"))]
pub mod app_ext;
#[cfg(feature = "bevy_async")]
mod async_commands;
// the app runner needs a sleep/yield + task pool, so it is std-only
#[cfg(all(feature = "bevy_async", feature = "std"))]
mod async_runner;
mod bevyhow;
mod common_systems;
mod despawn_after;
mod entity_target_event;
mod garbage_collect;
mod id_counter;
mod maybe;
mod non_send_marker;
mod non_send_plugin;

pub use bevyhow::*;
mod observer_adder;
#[cfg(feature = "std")]
pub mod observer_ext;
mod on_spawn;
#[cfg(feature = "std")]
mod pretty_tracing;
pub mod reflect_ext;

mod when;

pub use ancestor_query::*;
#[cfg(feature = "bevy_async")]
pub use async_commands::*;
#[cfg(all(feature = "bevy_async", feature = "std"))]
pub use async_runner::*;
pub use common_systems::*;
pub use despawn_after::*;
pub use entity_target_event::*;
pub use garbage_collect::*;
pub use id_counter::*;
pub use maybe::*;
pub use non_send_marker::*;
pub use non_send_plugin::*;
pub use on_spawn::*;
#[cfg(feature = "std")]
pub use pretty_tracing::*;
pub use when::*;
