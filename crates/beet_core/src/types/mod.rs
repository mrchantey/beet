//! Various widely used types

/// Bytes paired with a [`MediaType`], for typed content handling.
pub mod media_bytes;
pub use media_bytes::*;
#[cfg(feature = "serde")]
pub mod media_serde;
mod media_type;
pub use media_type::*;
