mod error;
#[cfg(not(target_arch = "wasm32"))]
mod impl_reqwest;
#[cfg(target_arch = "wasm32")]
mod impl_wasm;
mod request;
mod response;
pub use error::*;
use http::StatusCode;
pub use request::*;
pub use response::*;
