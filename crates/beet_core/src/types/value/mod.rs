#[cfg(target_arch = "wasm32")]
mod js;
#[cfg(feature = "json")]
mod json;
pub mod map;
pub use map::*;
#[cfg(feature = "serde")]
mod serde;
#[cfg(feature = "serde")]
pub use serde::DeError;
#[cfg(feature = "serde")]
pub use serde::SerError;
#[cfg(feature = "serde")]
pub use serde::ValueDeserializer;
#[cfg(feature = "serde")]
pub use serde::ValueSerializer;
mod value;
pub use value::*;
// writing a `Value` as json text without serde, so a generator whose output
// must exist in every build does not ride the `json` feature.
mod write_json;
