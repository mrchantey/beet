//! Infrastructure as code for beet, built on OpenTofu.
//!
//! This crate provides typed Rust bindings for Terraform/OpenTofu
//! configuration, including:
//! - [`config_exporter`] for building and exporting JSON configurations
//! - [`bindings_generator`] for generating typed Rust bindings from provider schemas
//! - [`common_resources`] for pre-generated bindings of commonly used providers
pub mod config_exporter;
mod types;

#[cfg(feature = "bindings_generator")]
pub mod bindings_generator;
pub mod common_resources;

/// Re-export so generated code can reference `crate::terra::*`
pub use config_exporter::types as terra;

pub mod prelude {
	pub use crate::config_exporter::config_exporter::*;
	pub use crate::config_exporter::types::*;
	pub use crate::types::*;
}
