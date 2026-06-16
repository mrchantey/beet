#![doc = include_str!("../README.md")]
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

pub use utils::async_ext;
pub use utils::time_ext;

// Re-export cross_log helpers at crate root so `$crate::` in macros resolves them.
// The platform checks live inside these functions (in beet_core where `std` is a
// declared feature), avoiding unexpected_cfgs warnings in downstream crates.
pub use utils::cross_log::_cross_log;
pub use utils::cross_log::_cross_log_error;
pub use utils::cross_log::_cross_log_noline;

#[cfg(feature = "std")]
pub mod arena;
mod bevy_extensions;
#[cfg(feature = "bsx")]
pub mod bsx;
pub mod bevy_utils;
pub mod extensions;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub mod fs;
mod path;
#[cfg(feature = "std")]
mod path_utils;
pub mod template;
// `term_style` (colours) is no_std and feeds the test logger, so the embedded
// test runner needs `terminal` too; the io/tty control parts stay std-gated
// inside the module.
#[cfg(any(feature = "std", feature = "testing_embedded"))]
pub mod terminal;
#[cfg(any(feature = "testing", feature = "testing_embedded"))]
pub mod testing;
#[cfg(feature = "tokens")]
pub mod tokens_utils;
pub mod types;
pub mod utils;
#[cfg(feature = "template_serde")]
pub mod template_serde;

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
#[cfg(feature = "custom_test_frameworks")]
pub use crate::testing::libtest_runner;
#[cfg(feature = "testing")]
pub use crate::testing::test_main;
#[cfg(feature = "testing")]
pub use crate::testing::test_runner;

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
	#[cfg(feature = "bsx")]
	pub use crate::bsx::*;
	pub use crate::bevybail;
	pub use crate::bevyhow;
	pub use crate::cfg_if;
	pub use crate::env_ext;
	pub use crate::extensions::*;
	#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
	pub use crate::fs::*;
	pub use crate::path::*;
	#[cfg(feature = "std")]
	pub use crate::path_utils::*;
	#[cfg(any(feature = "testing", feature = "testing_embedded"))]
	pub use crate::testing::*;
	pub use crate::subtree_template;
	pub use crate::template::*;
	#[cfg(feature = "tokens")]
	pub use crate::tokens_utils::*;
	pub use crate::types::*;
	pub use crate::utils::*;
	#[cfg(feature = "template_serde")]
	pub use crate::template_serde::*;
	#[cfg(any(feature = "std", feature = "testing_embedded"))]
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
	pub use bevy::platform::collections::HashMap;
	pub use bevy::platform::collections::HashSet;
	pub use bevy::platform::hash::FixedHasher;
	pub use bevy_ecs::entity_disabling::Disabled;
	// tracing's log macros are no_std, unlike bevy_log which is std-gated, so
	// re-export them directly to give downstream crates a cross-platform surface.
	pub use tracing::Level;
	pub use tracing::debug;
	pub use tracing::debug_span;
	pub use tracing::error;
	pub use tracing::error_span;
	pub use tracing::info;
	pub use tracing::info_span;
	pub use tracing::trace;
	pub use tracing::trace_span;
	pub use tracing::warn;
	pub use tracing::warn_span;

	#[cfg(target_arch = "wasm32")]
	pub use crate::js_runtime;

	pub use bevy::prelude::*;
	// no_std-capable integer-power helpers (`.squared()`/`.cubed()`); the
	// `ops` module (cross-platform `sin`/`cos`/…) already arrives via
	// `bevy::prelude`.
	pub use bevy::math::FloatPow;
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
	pub use core::time::Duration;
	pub use futures_lite::StreamExt;
	pub use smol_str::SmolStr;
	// `Instant` is no_std-capable: on std/wasm it wraps std/web_time, and on a
	// bare no_std target it reads from a clock source the embedded adapter
	// installs via `Instant::set_elapsed(...)` (see agent/plans/no_std_instant.md).
	pub use bevy::platform::time::Instant;
	// wall-clock helpers, incl. the `set_now`/`try_now` hook a no_std adapter
	// (eg an SNTP client) uses to supply time. Code that needs the std
	// `SystemTime` type imports `std::time::SystemTime` directly.
	pub use crate::time_ext;

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
	// the engine, re-exported so internal macros can emit
	// `beet_core::exports::bevy` and downstream crates need no direct `bevy` dep.
	pub use bevy;
	pub use itertools::Itertools;
	// original exports
	#[cfg(feature = "std")]
	pub use async_channel;
	pub use futures_lite;
	#[cfg(feature = "tokens")]
	pub use proc_macro2;
	#[cfg(feature = "tokens")]
	pub use quote;
	#[cfg(feature = "ron")]
	pub use ron;
	#[cfg(feature = "std")]
	pub use send_wrapper::SendWrapper;
	#[cfg(feature = "tokens")]
	pub use syn;

	// merged-in exports
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
