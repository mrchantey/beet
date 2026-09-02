//! Interface-oriented schema for [`Value`]s.
//!
//! Loosely parallels bevy's [`TypeInfo`](bevy::reflect::TypeInfo), but is
//! designed for driving dynamic UIs, validation and form generation.
//!
//! Convert from a bevy reflect type with [`ValueSchema::from_type_info`].
//! Run validation with [`ValueSchema::validate`].

mod constraint;
mod field_schema;
#[cfg(feature = "json")]
mod from_json;
mod from_type_info;
mod kinds;
mod schema_registry;
mod value_schema;
pub use constraint::*;
pub use field_schema::*;
pub use kinds::*;
pub use schema_registry::*;
pub use value_schema::*;
