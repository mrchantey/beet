//! HTTP client for cross-platform request sending.
//!
//! This module provides a unified HTTP client API that works on both native
//! and WASM targets. The implementation is selected at compile time based on
//! the target and enabled features:
//!
//! - **WASM**: Uses `web-sys` fetch API
//! - **Native with `ureq`**: Uses ureq (blocking, wrapped in unblock + async)
//! - **Native with `reqwest`**: Uses reqwest (async)
//!
//! # Example
//!
//! ```ignore
//! use beet_net::prelude::*;
//! use beet_core::prelude::*;
//!
//! async fn fetch_data() -> Result<String> {
//!     let response = Request::get("https://example.com")
//!         .send()
//!         .await?;
//!     response.text().await
//! }
//! ```
mod event_source;
#[cfg(all(feature = "reqwest", not(target_arch = "wasm32")))]
mod impl_reqwest;
#[cfg(all(feature = "ureq", not(target_arch = "wasm32")))]
mod impl_ureq;
#[cfg(target_arch = "wasm32")]
mod impl_web_sys;
// pub use event_source::*;
mod send;
pub use event_source::*;
pub use send::*;
