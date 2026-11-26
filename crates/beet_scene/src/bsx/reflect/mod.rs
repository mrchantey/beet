//! Literal to reflected value: type inference against a target's [`TypeInfo`],
//! and prop verification.
//!
//! A type's authored spelling is not described here: it is one
//! [`LiteralParser`](beet_core::prelude::LiteralParser) entry per type, and every
//! seam in this module follows that seam contract: build a `Value`, look the
//! entry up by the target's `TypeId`, and fall through to the structural rules
//! only when the parser declines. What lives here are those structural rules
//! and the dispatch between them.
//!
//! [`TypeInfo`]: bevy::reflect::TypeInfo

#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use beet_core::prelude::*;

mod composite;
mod literal;
mod scalar;
mod schema;

pub use literal::LiteralReflect;
pub(in crate::bsx) use literal::*;
pub(in crate::bsx) use schema::*;
