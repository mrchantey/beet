//! Shared utilities for `beet_core` and `beet_core_macros`.
//!
//! This crate contains token utilities that need to be compiled for both
//! the main `beet_core` crate and the `beet_core_macros` proc-macro crate.

mod attribute_group;
mod attribute_map;
mod named_field;
/// Package configuration extensions for Cargo.toml parsing.
pub mod pkg_ext;
mod synhow;

pub mod prelude {
	pub use crate::attribute_group::*;
	pub use crate::attribute_map::*;
	pub use crate::named_field::*;
	pub use crate::pkg_ext;
	pub use crate::synbail;
	pub use crate::synhow;
}
