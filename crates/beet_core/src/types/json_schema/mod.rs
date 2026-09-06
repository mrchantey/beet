//! JSON Schema interchange built from reflected Rust types.

mod from_type_info;
#[cfg(feature = "json")]
mod json;
mod json_schema;
pub use json_schema::*;
mod strict;
