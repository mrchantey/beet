//! An authored literal ([`DataLiteral`]) to the value a scene body holds,
//! shared by every authored source: markup, frontmatter, an office file's
//! metadata. [`LiteralValues`] lowers a literal to its type's serde form, and
//! [`LiteralReflect`] resolves it to a reflected value, its type inferred from
//! a target's [`TypeInfo`].
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
mod literal_reflect;
mod literal_values;
mod scalar;

pub use literal_reflect::LiteralReflect;
use literal_reflect::*;
pub use literal_values::*;
