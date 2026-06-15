//! Testing utilities for beet crates.
//!
//! This module provides a cross-platform test runner and matchers for writing tests.
//!
//! # Features
//!
//! - **Test matchers**: Type-specific matchers like `xpect_eq`, `xpect_close`, `xpect_contains`, etc.
//! - **Test runner**: Cross-platform test runner with wasm support and pretty output
//! - **Async tests**: Full support for async test functions via `#[beet_core::test]`
//! - **Per-test timeouts**: Configure timeouts at the test level
//! - **Snapshot testing**: Compare test output against saved snapshots
//!
//! # Usage
//!
//! ## Basic Tests
//!
//! ```rust,ignore
//! use beet_core::prelude::*;
//!
//! #[beet_core::test]
//! fn it_passes() {
//!     // regular assertions work as expected
//!     assert!(1 + 1 == 2);
//!     // type-specific matchers are also available
//!     "foobar".xpect_contains("foo");
//! }
//! ```
//!
//! ## Async Tests
//!
//! ```rust,ignore
//! #[beet_core::test]
//! async fn async_test() {
//!     beet_core::time_ext::sleep_millis(10).await;
//!     assert_eq!(2 + 2, 4);
//! }
//! ```
//!
//! ## Per-Test Timeouts
//!
//! Individual tests can specify custom timeout values:
//!
//! ```rust,ignore
//! #[beet_core::test(timeout_ms = 100)]
//! async fn quick_test() {
//!     // this test will timeout after 100ms instead of the default 5000ms
//!     beet_core::time_ext::sleep_millis(10).await;
//! }
//! ```
//!
//! ## Setup
//!
//! To use the test runner in your crate:
//!
//! 1. Add `beet_core` with the `testing` feature in `dev-dependencies`:
//!    ```toml
//!    [dev-dependencies]
//!    beet_core = { workspace = true, features = ["testing"] }
//!    ```
//!
//! 2. Set `harness = false` for the lib and every integration test target
//!    in `Cargo.toml`, e.g.:
//!    ```toml
//!    [lib]
//!    harness = false
//!    ```
//!
//! 3. Add the runner entry point once per lib / test target:
//!    ```rust,ignore
//!    beet_core::test_main!();
//!    ```
//!
//! 4. Import the prelude and write `#[beet_core::test]` tests:
//!    ```rust,ignore
//!    use beet_core::prelude::*;
//!    ```

mod matchers;
pub use matchers::*;
mod runner;
mod utils;
pub use runner::*;
pub use utils::*;

// Test registration is per-platform: native/wasm collect via `inventory`'s
// life-before-main constructors, but those never run on bare metal, so the
// embedded build registers into a `linkme` distributed slice instead. Both are
// driven by the same `submit!` the `#[beet_core::test]` macro emits; the macro
// is selected here by feature so the test author's usage is identical.
#[cfg(feature = "testing")]
#[doc(hidden)]
pub use inventory::submit as _inventory_submit;

/// linkme's distributed-slice attribute, re-exported so the `submit!` macro can
/// reference it by `$crate` path without the test author depending on linkme.
#[cfg(feature = "testing_embedded")]
#[doc(hidden)]
pub use linkme::distributed_slice;

/// Registers a [`BeetTestCase`] via `inventory` (native/wasm).
///
/// The leading ident (the unique static name linkme needs) is unused here.
#[cfg(feature = "testing")]
#[macro_export]
#[doc(hidden)]
macro_rules! __beet_testing_submit {
	($name:ident, $case:expr $(,)?) => {
		$crate::testing::_inventory_submit! { $case }
	};
}

/// Registers a [`BeetTestCase`] into the [`BEET_TESTS`] `linkme` slice (embedded).
#[cfg(all(feature = "testing_embedded", not(feature = "testing")))]
#[macro_export]
#[doc(hidden)]
macro_rules! __beet_testing_submit {
	($name:ident, $case:expr $(,)?) => {
		// The static name is derived from the (snake_case) test fn ident, so it
		// trips `non_upper_case_globals`; the name is an internal linkme handle.
		#[allow(non_upper_case_globals)]
		#[$crate::testing::distributed_slice($crate::testing::BEET_TESTS)]
		static $name: $crate::testing::BeetTestCase = $case;
	};
}

/// The `#[beet_core::test]` registration entry, switched per-platform above.
#[doc(hidden)]
pub use crate::__beet_testing_submit as submit;
