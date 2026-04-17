//! Native filesystem utilities for file watching and command execution.
//!
//! This module provides utilities for working with the native filesystem,
//! including file watching for hot-reloading and command execution helpers.
//!
//! # Features
//!
//! - [`FsWatcher`] - File system watcher with debouncing
//! - [`ChildProcess`] - Helper for spawning processes with stdout collection
//! - [`CargoBuildCmd`] - Builder for Cargo build commands
//! - [`CargoBuild`] - Slim builder for constructing [`BuildArtifact`]
//! - [`BuildArtifact`] - Build step component for deploy sequences
//! - [`Tempdir`] - Temporary directory management (requires `rand` feature)
//!
//! # Platform Support
//!
//! This module is only available on native platforms (not wasm).

mod cargo_build_cmd;
mod fs_watcher;
mod child_process;
mod cargo_build;
mod build_artifact;
#[cfg(feature = "rand")]
mod tempdir;

pub use cargo_build_cmd::*;
pub use fs_watcher::*;
pub use child_process::*;
pub use cargo_build::*;
pub use build_artifact::*;
#[cfg(feature = "rand")]
pub use tempdir::*;
