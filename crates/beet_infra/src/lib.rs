//! Infrastructure as code for beet, built on OpenTofu.
//!
//! This crate provides typed Rust bindings for Terraform/OpenTofu
//! configuration, including:
//! - [`terra`] for building and exporting JSON configurations
//! - [`bindings_generator`] for generating typed Rust bindings from provider schemas
//! - [`bindings`] for pre-generated bindings of commonly used providers

beet_core::test_main!();

#[cfg(feature = "deploy")]
mod actions;
pub mod bindings;
mod blocks;
pub mod terra;
mod types;
mod wasm;

#[cfg(feature = "bindings_generator")]
pub mod bindings_generator;

pub mod prelude {
	#[cfg(feature = "deploy")]
	pub use crate::actions::*;
	pub use crate::bindings;
	#[allow(unused)]
	pub use crate::blocks::*;
	pub use crate::terra;
	pub use crate::terra::tofu;
	pub use crate::types::*;
	pub use crate::wasm::*;
}
