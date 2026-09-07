//! Schema construction from external descriptions.

mod default_value;
mod from_json;
mod from_type_info;
mod meta_schema;
// the shared primitive vocabulary has exactly two consumers, both gated: the
// JSON Schema parser and the `schema=".."` markup attribute.
#[cfg(any(feature = "json", feature = "bsx"))]
mod primitive_name;
