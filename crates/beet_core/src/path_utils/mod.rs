//! Cross-platform filesystem abstractions: the std surface beneath the
//! `no_std` path types ([`SmolPath`], [`AbsPath`], [`WsPath`]).
//!
//! # Key Types
//!
//! - [`FsError`] - Filesystem operation error type, always naming the path
//! - [`ReadDir`] - Directory listing
//! - [`EnvVar`] - Serde and reflect-friendly environment variable
//!
//! # Modules
//!
//! - [`fs_ext`] - Cross-platform filesystem operations
//! - [`path_ext`](crate::path::path_ext) - Path cleaning and [`Path`](std::path::Path) helpers

mod env_var;
mod fs_error;
pub mod fs_ext;
mod read_dir;

pub use env_var::*;
pub use fs_error::*;
pub use read_dir::*;
