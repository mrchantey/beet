//! Various widely used types
#[cfg(feature = "serde")]
pub mod media_serde;
mod media_type;
pub use media_type::*;
