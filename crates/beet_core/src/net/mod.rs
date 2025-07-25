mod cross_fetch;
mod error;
#[cfg(not(target_arch = "wasm32"))]
mod impl_reqwest;
#[cfg(target_arch = "wasm32")]
mod impl_web_sys;
pub use error::*;
