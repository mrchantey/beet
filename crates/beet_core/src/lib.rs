#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(all(feature = "testing", not(test)), feature(test))]
#![cfg_attr(
	feature = "nightly",
	feature(fn_traits, unboxed_closures, never_type)
)]
// allow name collision until exit_ok stablized
#![allow(unstable_name_collisions)]

pub use utils::async_ext;
pub use utils::time_ext;

pub mod arena;
mod bevy_extensions;
mod bevy_utils;
pub mod extensions;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod fs;
mod path_utils;
#[cfg(feature = "testing")]
extern crate test;
#[cfg(feature = "testing")]
pub mod test_utils;
#[cfg(feature = "tokens")]
pub mod tokens_utils;
pub mod utils;

#[cfg(target_arch = "wasm32")]
pub mod web_utils;
pub use beet_core_macros::*;
#[cfg(target_arch = "wasm32")]
pub use web_utils::js_runtime;

mod workspace_config;

pub mod prelude {
	pub use crate::arena::*;
	pub use crate::bevy_extensions::*;
	pub use crate::bevy_utils::*;
	pub use crate::bevybail;
	pub use crate::bevyhow;
	pub use crate::extensions::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::*;
	pub use crate::path_utils::*;
	#[cfg(feature = "testing")]
	pub use crate::test_utils::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	pub use crate::utils::*;
	pub use either::Either;
	pub use std::marker::PhantomData;
	// as a metaframework we're a superset of bevy,
	// and more opinionated about kitchen sink prelude inclusions
	/// hack to fix bevy macros
	pub use bevy::ecs as bevy_ecs;
	pub use bevy::ecs::entity::MapEntities;
	pub use bevy::ecs::lifecycle::HookContext;
	pub use bevy::ecs::query::QueryData;
	pub use bevy::ecs::query::QueryFilter;
	pub use bevy::ecs::reflect::ReflectMapEntities;
	pub use bevy::ecs::relationship::Relationship;
	pub use bevy::ecs::schedule::ScheduleLabel;
	pub use bevy::ecs::system::RunSystemOnce;
	pub use bevy::ecs::system::SystemParam;
	pub use bevy::ecs::traversal::Traversal;
	pub use bevy::ecs::world::DeferredWorld;
	pub use bevy::log::LogPlugin;
	pub use bevy::platform::collections::HashMap;
	pub use bevy::platform::collections::HashSet;
	pub use bevy::platform::hash::FixedHasher;
	pub use bevy_ecs::entity_disabling::Disabled;
	pub use std::hash::BuildHasher;
	pub use tracing::Level;

	#[cfg(target_arch = "wasm32")]
	pub use crate::js_runtime;

	pub use bevy::prelude::*;
	/// hack to fix bevy macros
	pub use bevy::reflect as bevy_reflect;
	pub use bevy::time::Stopwatch;

	pub use crate::pkg_config;
	#[cfg(target_arch = "wasm32")]
	pub use crate::web_utils::*;
	pub use crate::workspace_config::*;
	pub use beet_core_macros::*;
	pub use futures_lite::StreamExt;
	pub use web_time::Duration;
	pub use web_time::Instant;

	pub use crate::abs_file;
	pub use crate::cross_log;
	pub use crate::cross_log_error;
	pub use crate::dir;
	#[cfg(feature = "rand")]
	pub use rand::Rng;
}

pub mod exports {
	pub use itertools::Itertools;
	// original exports
	pub use async_channel;
	pub use futures_lite;
	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
	#[cfg(feature = "serde")]
	pub use ron;
	pub use send_wrapper::SendWrapper;
	#[cfg(feature = "tokens")]
	pub use syn;
	#[cfg(feature = "serde")]
	pub use toml;
	pub use web_time;

	// merged-in exports
	pub use glob;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use notify;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use notify_debouncer_full;

	#[cfg(target_arch = "wasm32")]
	pub use js_sys;
	#[cfg(target_arch = "wasm32")]
	pub use wasm_bindgen;
	#[cfg(target_arch = "wasm32")]
	pub use wasm_bindgen_futures;
	#[cfg(target_arch = "wasm32")]
	pub use web_sys;
}
