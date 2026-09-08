//! Literal to reflected value: type inference against a target's [`TypeInfo`],
//! the root declaration block, and prop-schema verification.
//!
//! A type's authored spelling is not described here: it is one
//! [`LiteralParser`](crate::prelude::LiteralParser) entry per type, and every
//! seam in this module follows that seam contract: build a `Value`, look the
//! entry up by the target's `TypeId`, and fall through to the structural rules
//! only when the parser declines. What lives here are those structural rules
//! and the dispatch between them.
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
