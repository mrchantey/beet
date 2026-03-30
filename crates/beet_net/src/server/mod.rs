//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Tool<Request, Response>`.
//!
//! ## Implementations
//!
//! - **Mini HTTP**: Lightweight async-io TCP server (default for `server` feature)
//! - **Hyper**: Full-featured HTTP server (requires `hyper` feature)
//! - **Lambda**: AWS Lambda runtime adapter (requires `lambda` feature)
//!
//! The server backend is selected at compile time based on feature flags.
//! All implementations route requests through the tool-based exchange
//! pattern, allowing the same handler code to work in every environment.
mod cli_server;
pub use cli_server::*;
#[cfg(not(target_arch = "wasm32"))]
mod repl_server;
#[cfg(not(target_arch = "wasm32"))]
pub use repl_server::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod http_server;
#[cfg(all(feature = "hyper", not(target_arch = "wasm32")))]
mod hyper_server;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
mod lambda_server;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod mini_http_server;
mod server_plugin;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use http_server::*;
#[cfg(all(
	feature = "server",
	feature = "hyper",
	not(target_arch = "wasm32")
))]
pub use hyper_server::*;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
pub use lambda_server::*;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub use mini_http_server::*;
pub use server_plugin::*;
