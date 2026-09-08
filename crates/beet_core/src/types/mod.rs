//! Various widely used types.

mod value;
pub use value::*;
mod literal;
pub use literal::*;
mod field_path;
pub use field_path::*;
mod on_missing;
pub use on_missing::*;
pub mod schema;
pub use schema::*;
pub mod json_schema;
pub use json_schema::*;
/// Bytes paired with a [`MediaType`], for typed content handling.
mod media;
pub use media::*;
pub mod element;
pub use element::*;
pub mod snippet;
pub use snippet::*;
#[cfg(feature = "serde")]
mod document;
#[cfg(feature = "serde")]
pub use document::*;
