//! Derive macro utilities for code generation.
//!
//! This module provides utilities for implementing derive macros and
//! generating code from Rust types, including:
//!
//! - [`TemplateMacro`]: Template instantiation from macro input
//! - [`ConstructMacro`]: Component construction macros
//! - [`DeriveProps`]: Props derivation for component types
//! - [`DeriveBuildable`]: Builder pattern derivation
mod template_macro;
pub use template_macro::*;
mod construct_macro;
mod derive_attribute_block;
mod derive_buildable;
mod derive_flatten;
mod derive_props;
mod node_field;
pub use construct_macro::*;
pub use derive_attribute_block::*;
pub use derive_buildable::*;
pub use derive_flatten::*;
pub use derive_props::*;
pub use node_field::*;
