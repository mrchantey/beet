#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Infrastructure as code for beet, built on OpenTofu.
//!
//! This crate provides typed Rust bindings for Terraform/OpenTofu
//! configuration, including:
//! - [`config_exporter`] for building and exporting JSON configurations
//! - [`bindings_generator`] for generating typed Rust bindings from provider schemas
//! - [`common_resources`] for pre-generated bindings of commonly used providers
mod components;
mod stacks;
pub mod types;

#[cfg(feature = "bindings_generator")]
pub mod bindings_generator;
pub mod common_resources;

pub mod prelude {
	pub use crate::components::*;
	pub use crate::stacks::*;
	pub use crate::types::*;
}
