//! World serialization and deserialization, forked from `bevy_world_serialization`.
//!
//! Unlike the upstream crate, this fork drops asset, camera, and `bevy_asset` dependencies,
//! keeping it lean and `no_std` compatible for embedded targets like the ESP32.
//!
//! # High-level API
//!
//! - [`WorldSerdeSaver`] - Serialize a world or entity subtree to RON, JSON, or postcard.
//! - [`WorldSerdeLoader`] - Deserialize bytes back into a world.
//!
//! # Building blocks
//!
//! - [`DynamicWorld`] - A reflection-backed snapshot of entities and resources.
//! - [`DynamicWorldBuilder`] - Extracts a [`DynamicWorld`] from a [`World`](bevy::prelude::World).
//! - [`WorldFilter`] - Allow/deny lists controlling which types are extracted.
//! - [`DynamicWorldSerializer`] / [`WorldDeserializer`] - The Serde implementations.

mod dynamic_world;
mod dynamic_world_builder;
mod loader;
mod reflect_utils;
mod saver;
// kept private so the `serde` name does not shadow the `serde` crate when this
// module is glob re-exported into the crate prelude
mod serde;
mod world_filter;

pub use dynamic_world::*;
pub use dynamic_world_builder::*;
pub use loader::*;
pub use saver::*;
pub use serde::DynamicWorldSerializer;
pub use serde::WorldDeserializer;
pub use world_filter::*;
