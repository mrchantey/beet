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
//! # Stable vs Nightly
//!
//! The test runner supports two modes:
//!
//! ## Stable (default)
//!
//! Tests are registered via `inventory` and collected at runtime.
//! The `#[test]` macro from `use beet_core::prelude::*` handles registration
//! automatically.
//!
//! ```rust,ignore
//! use beet_core::prelude::*;
//!
//! #[test]
//! fn it_passes() {
//!     "foobar".xpect_contains("foo");
//! }
//! ```
//!
//! ## Nightly (`custom_test_framework` feature)
//!
//! Uses the unstable `custom_test_frameworks` feature for tighter integration
//! with the Rust test harness.
//!
//! ```rust,ignore
//! #![cfg_attr(test, feature(test, custom_test_frameworks))]
//! #![cfg_attr(test, test_runner(beet_core::test_runner_nightly))]
//! use beet_core::prelude::*;
//!
//! #[test]
//! fn it_passes() {
//!     "foobar".xpect_contains("foo");
//! }
//! ```

mod matchers;
pub use matchers::*;
mod runner;
mod utils;
pub use runner::*;
pub use utils::*;

// Re-export the nightly test runner entry point
#[cfg(feature = "custom_test_framework")]
pub use runner::test_runner_nightly;

// Re-export the stable test runner entry point
pub use runner::test_runner;
