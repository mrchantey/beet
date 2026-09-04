//! Interface-oriented schema for [`Value`]s.
//!
//! Loosely parallels bevy's [`TypeInfo`](bevy::reflect::TypeInfo), but is
//! designed for driving dynamic UIs, validation and form generation.
//!
//! Convert from a bevy reflect type with [`ValueSchema::from_type_info`].
//! Run validation with [`ValueSchema::validate`].

mod constraint;
#[cfg(feature = "json")]
mod from_json;
mod from_type_info;
mod kinds;
mod meta_schema;
mod on_missing;
mod schema_commit;
mod schema_ref;
mod schema_registry;
mod schema_resolver;
mod value_schema;
pub use constraint::*;
pub use kinds::*;
pub use on_missing::*;
pub use schema_commit::*;
pub use schema_ref::*;
pub use schema_registry::*;
pub use schema_resolver::*;
pub use value_schema::*;
