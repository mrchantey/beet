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
//! Connections are pooled by every backend, so a run of calls to one host
//! pays its TCP and TLS handshake once:
//!
//! - **ureq**: one static agent, the redirect budget set per request.
//! - **reqwest**: static clients, one per common redirect policy; an unusual
//!   hop budget builds its own.
//! - **fetch** (a Worker, Deno, a browser): the host's own pool.
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
// AWS as Signature Version 4 requests over this client
#[cfg(feature = "aws")]
mod aws;
#[cfg(feature = "http")]
mod event_source;
#[cfg(feature = "fs")]
mod impl_file;
// `send_http` prefers ureq when both are enabled, so gating here keeps the
// unreachable transport out of the build rather than compiling it dead.
#[cfg(all(
	feature = "reqwest",
	not(feature = "ureq"),
	not(target_arch = "wasm32")
))]
mod impl_reqwest;
// ureq shadows reqwest, which stays linked: mark it used for the unused
// dependency lint
#[cfg(all(
	feature = "reqwest",
	feature = "ureq",
	not(target_arch = "wasm32")
))]
use reqwest as _;
#[cfg(all(feature = "ureq", not(target_arch = "wasm32")))]
mod impl_ureq;
// the fetch transport rides `std`: its response stream goes through the
// std-gated `futures` channel and `send_wrapper`.
#[cfg(all(target_arch = "wasm32", feature = "std"))]
mod impl_web_sys;
#[cfg(feature = "std")]
mod request_auth;
mod send;
#[cfg(feature = "aws")]
pub use aws::*;
#[cfg(feature = "http")]
pub use event_source::*;
#[cfg(feature = "fs")]
pub use impl_file::*;
#[cfg(feature = "std")]
pub use request_auth::*;
pub use send::*;
