//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via the exchange system.
//!
//! ## Implementations
//!
//! - **Hyper**: Default async HTTP server (requires `server` feature)
//! - **Lambda**: AWS Lambda runtime adapter (requires `lambda` feature)
//!
//! The server backend is selected at compile time based on feature flags.
//! Both implementations route requests through [`ExchangeStart`] events,
//! allowing the same handler code to work in both environments.
//!
//! [`ExchangeStart`]: crate::prelude::ExchangeStart
mod cli_server;
#[cfg(all(
	feature = "server",
	not(feature = "lambda"),
	not(target_arch = "wasm32")
))]
mod hyper_server;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
mod lambda_server;

#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod http_server;
mod server_plugin;
pub use cli_server::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use http_server::*;
#[cfg(all(
	feature = "server",
	not(feature = "lambda"),
	not(target_arch = "wasm32")
))]
use hyper_server::*;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
use lambda_server::*;
pub use server_plugin::*;
