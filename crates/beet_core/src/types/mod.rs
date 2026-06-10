//! Various widely used types

/// Bytes paired with a [`MediaType`], for typed content handling.
mod value;
pub use value::*;
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
