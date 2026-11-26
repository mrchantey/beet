//! Schema construction from external descriptions.

mod default_value;
mod from_type_info;
mod from_value;
mod meta_schema;
// shared by the JSON Schema parser and the `schema=".."` markup attribute
mod primitive_name;
