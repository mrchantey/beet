//! Core utilities and types for the beet framework.
//!
//! This crate provides foundational building blocks used across all beet crates,
//! including cross-platform abstractions, extension traits, and common patterns.
//!
//! # Modules
//!
//! - [`arena`] - Global arenas for storing objects with copyable handles (requires `std`)
//! - [`extensions`] - Extension traits for standard library types
//! - [`utils`] - General utilities including async helpers and method chaining
//! - [`testing`] - Custom test runner and matchers (requires `testing` feature)
//! - [`fs`] - File system utilities (native only, requires `fs` feature)
//!
//! # Feature Flags
//!
//! - `std` - Standard library support (enabled by default)
//! - `serde` - Serialization support
//! - `testing` - Test runner and matcher utilities
//! - `fs` - File system watching and utilities (native only)
//! - `tokens` - Proc-macro token utilities
//! - `rand` - Random number generation
//! - `nightly` - Nightly Rust features like `Fn` trait implementations

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
// The lib itself always uses the stable `inventory` runner (`[lib]
// harness = false`). The libtest / `custom_test_frameworks` path is only
// exercised by `tests/test_test.rs`. See
// `crates/beet_core/src/testing/runner/test_desc.rs`.
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
// `extern crate test` (the libtest conversion shims) requires the unstable
// `test` feature, on both test and non-test builds.
#![cfg_attr(feature = "custom_test_frameworks", feature(test))]
// allow name collision until exit_ok stablized
#![allow(unstable_name_collisions)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

/// Re-export of [`alloc`] for use in macros across std/no_std boundaries.
#[doc(hidden)]
pub extern crate alloc as _alloc;

#[cfg(feature = "custom_test_frameworks")]
extern crate test;

#[cfg(feature = "std")]
pub use utils::async_ext;
#[cfg(feature = "std")]
pub use utils::time_ext;

// Re-export cross_log helpers at crate root so `$crate::` in macros resolves them.
// The std feature check lives inside these functions (in beet_core where std is a
// declared feature), avoiding unexpected_cfgs warnings in downstream crates.
#[cfg(not(target_arch = "wasm32"))]
pub use utils::cross_log::_cross_log_error_native;
#[cfg(not(target_arch = "wasm32"))]
pub use utils::cross_log::_cross_log_native;
#[cfg(not(target_arch = "wasm32"))]
pub use utils::cross_log::_cross_log_native_noline;

#[cfg(feature = "std")]
pub mod arena;
mod bevy_extensions;
pub mod bevy_utils;
pub mod extensions;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod fs;
#[cfg(feature = "std")]
mod path_utils;
#[cfg(feature = "std")]
pub mod terminal;
#[cfg(feature = "testing")]
pub mod testing;
#[cfg(feature = "tokens")]
pub mod tokens_utils;
pub mod types;
pub mod utils;
#[cfg(feature = "world_serde")]
pub mod world_serde;

#[cfg(target_arch = "wasm32")]
pub mod web_utils;
// Re-export for ergonomic `#[beet_core::test]` and `#[beet_core::main]` usage
pub use beet_core_macros::beet_main as main;
pub use beet_core_macros::beet_test as test;
pub use beet_core_macros::*;
#[cfg(target_arch = "wasm32")]
pub use web_utils::js_runtime;

#[cfg(feature = "std")]
mod workspace_config;
#[cfg(feature = "testing")]
pub use crate::testing::test_runner;
#[cfg(feature = "testing")]
pub use crate::testing::test_main;
#[cfg(feature = "custom_test_frameworks")]
pub use crate::testing::libtest_runner;

/// Entry point for a `harness = false` test target / lib.
///
/// Expands to a `#[cfg(test)] fn main` that runs every `#[beet_core::test]`
/// registered via [`inventory`]. Place once per lib and integration test.
///
/// Always defined (independent of the `testing` feature) so non-test builds
/// of downstream crates can still resolve the macro; the generated `fn main`
/// is `#[cfg(test)]`-gated, and dev-dependencies enable `testing` for tests.
#[macro_export]
macro_rules! test_main {
	() => {
		#[cfg(test)]
		fn main() { $crate::testing::test_main(); }
	};
}

// beet_core's own `cargo test --lib` entry point (`[lib] harness = false`).
// The lib always uses the inventory runner; the libtest path is only for
// `tests/test_test.rs`.
#[cfg(all(test, feature = "testing"))]
fn main() { crate::testing::test_main(); }

/// Re-exports of commonly used types and traits.
///
/// This prelude is designed to be glob-imported for convenience:
///
/// ```ignore
/// use beet_core::prelude::*;
/// ```
pub mod prelude {
	pub use crate::val;

	// Re-export alloc types so modules using `crate::prelude::*` get them
	// regardless of std/no_std. This avoids scattering `use alloc::*`
	// throughout this crate and downstream no_std crates.
	pub use alloc::boxed::Box;
	pub use alloc::format;
	pub use alloc::string::String;
	pub use alloc::string::ToString;
	pub use alloc::vec;
	pub use alloc::vec::Vec;
	pub use core::hash::BuildHasher;
	pub use core::marker::PhantomData;
	/// Shorthand for `Ok(())`
	pub const OK: Result<(), BevyError> = Ok(());
	pub use core::ops::ControlFlow;

	#[cfg(feature = "std")]
	pub use crate::arena::*;
	pub use crate::bevy_extensions::*;
	pub use crate::bevy_utils::*;
	pub use crate::bevybail;
	pub use crate::bevyhow;
	pub use crate::cfg_if;
	pub use crate::extensions::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::*;
	#[cfg(feature = "std")]
	pub use crate::path_utils::*;
	#[cfg(feature = "testing")]
	pub use crate::testing::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	pub use crate::types::*;
	pub use crate::utils::*;
	#[cfg(feature = "world_serde")]
	pub use crate::world_serde::*;
	// `DynamicWorld`, `DynamicWorldBuilder` and `WorldFilter` also exist in
	// `bevy::prelude` when `bevy_world_serialization` is enabled (eg via
	// `bevy/default`). Re-export ours explicitly so they shadow bevy's globs,
	// keeping beet's fork canonical and avoiding ambiguous glob re-exports.
	#[cfg(feature = "world_serde")]
	pub use crate::world_serde::DynamicWorld;
	#[cfg(feature = "world_serde")]
	pub use crate::world_serde::DynamicWorldBuilder;
	#[cfg(feature = "world_serde")]
	pub use crate::world_serde::WorldFilter;
	#[cfg(feature = "std")]
	pub use crate::terminal::*;
	pub use either::Either;
	#[cfg(feature = "serde")]
	pub use serde::Deserialize;
	#[cfg(feature = "serde")]
	pub use serde::Serialize;
	#[cfg(feature = "serde")]
	pub use serde::de::DeserializeOwned;
	// as a metaframework we're a superset of bevy,
	// and more opinionated about kitchen sink prelude inclusions
	/// hack to fix bevy macros
	pub use bevy::ecs as bevy_ecs;
	pub use bevy::ecs::entity::MapEntities;
	pub use bevy::ecs::lifecycle::HookContext;
	pub use bevy::ecs::query::Allow;
	pub use bevy::ecs::query::QueryData;
	pub use bevy::ecs::query::QueryFilter;
	pub use bevy::ecs::reflect::ReflectMapEntities;
	pub use bevy::ecs::relationship::Relationship;
	pub use bevy::ecs::schedule::ScheduleLabel;
	pub use bevy::ecs::system::RunSystemOnce;
	pub use bevy::ecs::system::SystemParam;
	pub use bevy::ecs::traversal::Traversal;
	pub use bevy::ecs::world::DeferredWorld;
	// bevy_log is std-only and only enabled via the `std` feature.
	#[cfg(feature = "std")]
	pub use bevy::log::LogPlugin;
	pub use bevy::platform::collections::HashMap;
	pub use bevy::platform::collections::HashSet;
	pub use bevy::platform::hash::FixedHasher;
	pub use bevy_ecs::entity_disabling::Disabled;
	pub use tracing::Level;

	#[cfg(target_arch = "wasm32")]
	pub use crate::js_runtime;

	pub use bevy::prelude::*;
	/// hack to fix bevy macros
	pub use bevy::reflect as bevy_reflect;
	pub use bevy::time::Stopwatch;

	#[cfg(feature = "bevy_color")]
	pub use bevy::color::palettes;

	#[cfg(feature = "std")]
	pub use crate::pkg_config;
	#[cfg(target_arch = "wasm32")]
	pub use crate::web_utils::*;
	#[cfg(feature = "std")]
	pub use crate::workspace_config::*;
	pub use beet_core_macros::*;
	pub use futures_lite::StreamExt;
	pub use smol_str::SmolStr;
	pub use core::time::Duration;
	// web_time provides Instant/SystemTime by wrapping std (or JS on wasm);
	// neither has a core-only equivalent, so they are std-gated.
	#[cfg(feature = "std")]
	pub use web_time::Instant;
	#[cfg(feature = "std")]
	pub use web_time::SystemTime;

	#[cfg(feature = "std")]
	pub use crate::abs_file;
	pub use crate::cross_log;
	pub use crate::cross_log_error;
	pub use crate::cross_log_noline;
	#[cfg(feature = "std")]
	pub use crate::dir;
	#[cfg(feature = "rand")]
	pub use rand::Rng;
}

/// Re-exports of external crates used by beet.
///
/// These exports allow downstream crates to use the same versions of
/// dependencies without adding them to their own `Cargo.toml`.
pub mod exports {
	pub use itertools::Itertools;
	// original exports
	#[cfg(feature = "std")]
	pub use async_channel;
	pub use futures_lite;
	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
	#[cfg(feature = "serde")]
	pub use ron;
	#[cfg(feature = "std")]
	pub use send_wrapper::SendWrapper;
	#[cfg(feature = "tokens")]
	pub use syn;
	#[cfg(feature = "std")]
	pub use web_time;

	// merged-in exports
	#[cfg(feature = "std")]
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
