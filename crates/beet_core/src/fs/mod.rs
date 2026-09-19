//! Native filesystem utilities for file watching and command execution.
//!
//! This module provides utilities for working with the native filesystem,
//! including file watching for hot-reloading and command execution helpers.
//!
//! # Features
//!
//! - [`FsWatcher`] - File system watcher with debouncing
//! - [`TempDir`] - Temporary directory management
//!
//! [`ChildProcess`](crate::prelude::ChildProcess) lives in
//! [`bootstrap`](crate::bootstrap): its description compiles everywhere
//!
//! # Platform Support
//!
//! This module is only available on native platforms (not wasm).

mod fs_watcher;
mod tempdir;

pub use fs_watcher::*;
pub use tempdir::*;
