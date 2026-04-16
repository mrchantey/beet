#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
//! Infrastructure as code for beet, built on OpenTofu.
//!
//! This crate provides typed Rust bindings for Terraform/OpenTofu
//! configuration, including:
//! - [`terra`] for building and exporting JSON configurations
//! - [`bindings_generator`] for generating typed Rust bindings from provider schemas
//! - [`bindings`] for pre-generated bindings of commonly used providers
pub mod bindings;
mod types;
mod actions;
mod blocks;
pub mod terra;

#[cfg(feature = "bindings_generator")]
pub mod bindings_generator;

pub mod prelude {
	#[allow(unused)]
	pub use crate::actions::*;
	pub use crate::bindings;
	pub use crate::types::*;
	#[allow(unused)]
	pub use crate::blocks::*;
	pub use crate::terra;
	pub use crate::terra::tofu;
}
