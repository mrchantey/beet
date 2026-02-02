//! Build actions for CLI commands, server management, and code generation.
//!
//! This module provides the core build-time actions including:
//! - CLI command execution and argument parsing
//! - Development server management
//! - Source file watching and hot reloading
//! - WASM compilation
//! - Lambda deployment
//! - Bucket synchronization

mod file_expr_changed;
pub use file_expr_changed::*;
mod server;
pub use server::*;
mod source_files;
pub use source_files::*;
mod sst;
pub use sst::*;
mod run_on_dir_event;
pub use run_on_dir_event::*;
mod lambda;
pub use lambda::*;
mod wasm;
pub use wasm::*;
mod command;
pub use command::*;
mod launch_config;
pub use launch_config::*;
mod sync_buckets;
pub use sync_buckets::*;
mod cli_plugin;
pub use beet_cli::*;
pub use cli_plugin::*;
mod beet_cli;
