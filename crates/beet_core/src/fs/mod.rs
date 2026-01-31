//! Native filesystem utilities for file watching and command execution.
//!
//! This module provides utilities for working with the native filesystem,
//! including file watching for hot-reloading and command execution helpers.
//!
//! # Features
//!
//! - [`FsWatcher`] - File system watcher with debouncing
//! - [`CommandExt`] - Extension trait for [`std::process::Command`]
//! - [`CargoBuildCmd`] - Builder for Cargo build commands
//! - [`Tempdir`] - Temporary directory management (requires `rand` feature)
//!
//! # Platform Support
//!
//! This module is only available on native platforms (not wasm).

mod cargo_build_cmd;
mod command_ext;
mod fs_watcher;
#[cfg(feature = "rand")]
mod tempdir;

pub use cargo_build_cmd::*;
pub use command_ext::*;
pub use fs_watcher::*;
#[cfg(feature = "rand")]
pub use tempdir::*;
