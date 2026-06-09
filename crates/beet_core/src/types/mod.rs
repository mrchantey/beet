//! Various widely used types

/// Bytes paired with a [`MediaType`], for typed content handling.
mod value;
pub use value::*;
mod media;
pub use media::*;
#[cfg(feature = "serde")]
mod document;
#[cfg(feature = "serde")]
pub use document::*;
