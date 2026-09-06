//! Interface-oriented schema for [`Value`]s.
//!
//! Loosely parallels bevy's [`TypeInfo`](bevy::reflect::TypeInfo), but is
//! designed for driving dynamic UIs, validation and form generation.
//!
//! Convert from a bevy reflect type with [`ValueSchema::from_type_info`].
//! Run validation with [`ValueSchema::validate`].

mod construction;
mod model;
mod resolution;
mod schema_commit;
mod validation;
pub use model::*;
pub use resolution::*;
pub use schema_commit::*;
pub use validation::*;
