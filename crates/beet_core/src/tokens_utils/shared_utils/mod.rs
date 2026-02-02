//! Token utilities shared by both `beet_core` and `beet_core_macros`
// Be careful with imports etc in this module, they must be compiled
// for both crates.
#![allow(unused)]
mod attribute_group;
/// Package configuration extensions for Cargo.toml parsing.
pub mod pkg_ext;
pub use attribute_group::*;
mod named_field;
pub use named_field::*;
