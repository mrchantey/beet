//! Typed Rust binding generation from OpenTofu provider schemas.
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
