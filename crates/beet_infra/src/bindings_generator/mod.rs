//! Typed Rust binding generation from OpenTofu provider schemas.
//!
//! These types serve for type safety when working on configurations,
//! they serve as a first line of defence before running `opentofu`,
//! which will also check the generated config for validity.
//!
//!
mod binding;
mod binding_generator;
mod config;
mod emit;
mod ir;
mod schema_binding_generator;
pub use binding::*;
pub use binding_generator::*;
pub use config::*;
pub use emit::*;
pub use ir::*;
pub use schema_binding_generator::*;
#[cfg(test)]
pub(self) mod test_utils;
