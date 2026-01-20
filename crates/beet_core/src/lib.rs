#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(crate::test_runner))]
#![cfg_attr(
	feature = "nightly",
	feature(
		fn_traits,
		unboxed_closures,
		never_type,
		async_fn_track_caller,
		closure_track_caller
	)
)]
// The test crate is needed for the test runner infrastructure
// Note: `test` feature is already enabled by cfg(test) above, so only add if_let_guard here
#![cfg_attr(feature = "testing", feature(if_let_guard))]
// never_type is needed for IntoFut impl for Future<Output = !>
// Only enable if nightly feature is not already enabling it
#![cfg_attr(
	all(feature = "testing", not(feature = "nightly")),
	feature(never_type)
)]
// Enable test feature for non-test builds that use the testing feature (e.g., other crates)
#![cfg_attr(all(feature = "testing", not(test)), feature(test))]
// allow name collision until exit_ok stablized
#![allow(unstable_name_collisions)]

#[cfg(feature = "testing")]
extern crate test;

pub use utils::async_ext;
pub use utils::time_ext;

pub mod arena;
mod bevy_extensions;
mod bevy_utils;
#[cfg(feature = "exchange")]
mod exchange;
pub mod extensions;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod fs;
mod path_utils;
#[cfg(feature = "testing")]
pub mod testing;
#[cfg(feature = "tokens")]
pub mod tokens_utils;
pub mod utils;

#[cfg(target_arch = "wasm32")]
pub mod web_utils;
/// Re-export sweet_test as test for ergonomic `#[beet_core::test]` usage
pub use beet_core_macros::beet_test as test;
pub use beet_core_macros::*;
#[cfg(target_arch = "wasm32")]
pub use web_utils::js_runtime;

mod workspace_config;
#[cfg(feature = "testing")]
pub use crate::testing::test_runner;

pub mod prelude {
	pub use crate::arena::*;
	pub use crate::bevy_extensions::*;
	pub use crate::bevy_utils::*;
	pub use crate::bevybail;
	pub use crate::bevyhow;
	#[cfg(feature = "exchange")]
	pub use crate::exchange::*;
	pub use crate::extensions::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::*;
	pub use crate::path_utils::*;
	#[cfg(feature = "testing")]
	pub use crate::testing::*;
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
