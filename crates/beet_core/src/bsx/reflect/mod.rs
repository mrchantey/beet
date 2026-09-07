//! Literal to reflected value: type inference against a target's [`TypeInfo`],
//! the root declaration block, and prop-schema verification.
//!
//! [`TypeInfo`]: bevy::reflect::TypeInfo

mod composite;
mod literal;
mod root_declarations;
mod scalar;
pub(in crate::bsx) mod schema;

pub(in crate::bsx) use literal::*;
pub use root_declarations::*;
pub(in crate::bsx) use schema::*;
