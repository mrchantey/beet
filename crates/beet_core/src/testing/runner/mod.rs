//! Test runner infrastructure for beet.
//!
//! This module provides a custom test runner that integrates with Bevy's
//! async runtime and provides enhanced test output formatting.

mod exit_on_suite_outcome;
mod insert_tests;
mod register_test;
mod suite_outcome;
mod test_outcome;
mod test_plugin;
pub use exit_on_suite_outcome::*;
pub use insert_tests::*;
pub use register_test::*;
pub use suite_outcome::*;
pub use test_outcome::*;
pub use test_plugin::*;
mod run_tests;
pub(self) use register_test::TestRunResult;
pub(self) use run_tests::*;
mod filter_tests;
pub use filter_tests::*;
mod logger;
pub(self) use logger::*;
/// Extensions for the test runner configuration.
pub mod test_runner_ext;
