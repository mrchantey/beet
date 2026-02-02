//! Testing utility types and functions.

/// Pretty formatting for libtest output.
pub mod run_libtest_pretty;
mod test_desc_ext;
/// Helpers for creating and manipulating test descriptors.
pub mod test_ext;
mod test_fut;
pub use test_desc_ext::*;
pub use test_fut::*;
/// Panic handling utilities.
pub mod panic_ext;
/// Utilities for testing panics across file boundaries.
pub mod panic_in_other_file;
/// Pretty diff formatting for snapshot comparisons.
pub mod pretty_diff;
