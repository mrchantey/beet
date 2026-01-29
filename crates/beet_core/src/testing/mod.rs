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
//! ```rust
//! #![cfg_attr(test, feature(test, custom_test_frameworks))]
//! #![cfg_attr(test, test_runner(beet_core::test_runner))]
//! use beet_core::prelude::*;
//!
//! #[test]
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
//! ```rust
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
//! ```rust
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
//! 2. Add the following attributes to your test files:
//!    ```rust
//!    #![cfg_attr(test, feature(test, custom_test_frameworks))]
//!    #![cfg_attr(test, test_runner(beet_core::test_runner))]
//!    ```
//!
//! 3. Import the prelude:
//!    ```rust
//!    use beet_core::prelude::*;
//!    ```

mod matchers;
pub use matchers::*;
mod runner;
mod utils;
pub use runner::*;
pub use utils::*;
