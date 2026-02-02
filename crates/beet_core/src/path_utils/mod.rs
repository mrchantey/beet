//! Path manipulation utilities and cross-platform filesystem abstractions.
//!
//! This module provides types and utilities for working with filesystem paths
//! in a cross-platform manner, including workspace-relative paths, absolute
//! paths, and environment variable access.
//!
//! # Key Types
//!
//! - [`WsPathBuf`] - Workspace-relative path with easy conversion to absolute
//! - [`AbsPathBuf`] - Guaranteed absolute path
//! - [`WorkspaceRoot`] - Bevy resource holding the workspace root directory
//! - [`FsError`] - Filesystem operation error type
//!
//! # Modules
//!
//! - [`env_ext`] - Cross-platform environment variable access
//! - [`fs_ext`] - Cross-platform filesystem operations
//! - [`path_ext`] - Extension traits for [`Path`](std::path::Path)

mod abs_path_buf;
/// Cross-platform environment variable access.
pub mod env_ext;
mod fs_error;
pub mod fs_ext;
pub mod path_ext;
mod read_dir;
mod workspace_root;
mod ws_path_buf;

pub use abs_path_buf::*;
pub use fs_error::*;
pub use read_dir::*;
pub use workspace_root::*;
pub use ws_path_buf::*;
