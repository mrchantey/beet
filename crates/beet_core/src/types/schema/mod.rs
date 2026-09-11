//! Interface-oriented schema for [`Value`](crate::types::Value)s.
//!
//! Loosely parallels bevy's [`TypeInfo`](bevy::reflect::TypeInfo), but is
//! designed for driving dynamic UIs, validation and form generation.
//!
//! Convert from a bevy reflect type with [`ValueSchema::from_type_info`], the
//! one reflection interpretation: JSON Schema is exported from what it produced
//! ([`JsonSchema::try_from_schema`](crate::types::JsonSchema::try_from_schema)),
//! not walked separately.
//! Run validation with [`ValueSchema::validate`].

mod construction;
mod model;
mod resolution;
mod scene;
mod schema_commit;
mod validation;
pub use model::*;
pub use resolution::*;
pub use scene::*;
pub use schema_commit::*;
pub use validation::*;
