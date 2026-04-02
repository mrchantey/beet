#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Infrastructure as code for beet, built on OpenTofu.
//!
//! This crate provides typed Rust bindings for Terraform/OpenTofu
//! configuration, including:
//! - [`terra_config`] for building and exporting JSON configurations
//! - [`bindings_generator`] for generating typed Rust bindings from provider schemas
//! - [`common_resources`] for pre-generated bindings of commonly used providers
pub mod bindings;
mod components;
mod stacks;
mod types;

#[cfg(feature = "bindings_generator")]
pub mod bindings_generator;

pub mod prelude {
	pub use crate::bindings;
	pub use crate::components::*;
	pub use crate::stacks::*;
	pub use crate::types::*;
	pub(crate) use smol_str::SmolStr;
}
