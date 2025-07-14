#![allow(async_fn_in_trait)]
pub mod cross_fetch;
#[cfg(not(target_arch = "wasm32"))]
mod reqwest;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::net::reqwest::*;
