//! Extension traits for Bevy types.
//!
//! This module provides additional methods on Bevy's core types through
//! extension traits, enabling more ergonomic APIs for common operations.
//!
//! # App Extensions
//!
//! - [`BeetCoreAppExt`] - Plugin initialization, async execution, time control
//!
//! # World Extensions
//!
//! - [`WorldExt`] - Local execution, entity inspection
//! - [`WorldMutExt`] - Query utilities, component inspection, scene building
//! - [`CoreWorldExt`] - Observer helpers
//!
//! # Entity Extensions
//!
//! - [`EntityExt`] - Entity lookup and relationship traversal
//!
//! # Transform Extensions
//!
//! - [`QuatExt`] - Quaternion utilities
//! - [`Vec3Ext`] - Vector utilities
//! - [`TransformExt`] - Transform manipulation

mod app;
mod app_exit;
#[cfg(feature = "bevy_color")]
mod color;
mod commands;
mod entity;
mod hierarchy;
#[cfg(feature = "bevy_keyboard")]
mod key;
mod plugin;
mod pose;
mod quat;
mod relationship;
mod schedule;
mod system;
mod transform_x;
mod vec3;
mod world;

pub use app::*;
pub use app_exit::*;
#[cfg(feature = "bevy_color")]
pub use color::*;
pub use commands::*;
pub use entity::*;
pub use hierarchy::*;
#[cfg(feature = "bevy_keyboard")]
pub use key::*;
pub use plugin::*;
pub use pose::*;
pub use quat::*;
pub use relationship::*;
pub use schedule::*;
pub use vec3::*;
pub use world::*;
