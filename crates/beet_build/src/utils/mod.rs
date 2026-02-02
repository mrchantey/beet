//! Build utilities and plugins.
//!
//! Provides core infrastructure for the build system:
//! - [`BuildPlugin`]: Main plugin for build-time operations
//! - [`CodegenFile`]: Representation of generated code files

mod build_plugin;
pub use build_plugin::*;
mod codegen_file;
pub use codegen_file::*;
