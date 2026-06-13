//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Action<Request, Response>`.
//!
//! ## Every binary is a CLI server
//!
//! A formal beet binary boots as a CLI server at the top level: its entrypoint
//! is a [`CliServer`] that parses argv into a [`Request`] and runs one exchange.
//! Long-running backends ([`HttpServer`], the `beet_router` `TuiServer`) are
//! never self-booting; the [`Server`] orchestrator starts them, pulled in by each
//! backend's `#[require(Server)]`. See [`server_backend`] for the model.
//!
//! ## Implementations
//!
//! - **Mini HTTP**: Lightweight async-io TCP server (default for `server` feature)
//! - **Hyper**: Full-featured HTTP server (requires `hyper` feature)
//! - **Lambda**: AWS Lambda runtime adapter (requires `lambda` feature)
//! - **Installed**: a backend supplied at runtime via [`set_http_server`], used
//!   on `no_std` targets with no compiled-in backend.
//!
//! The server backend is selected at compile time based on feature flags.
//! All implementations route requests through the action-based exchange
//! pattern, allowing the same handler code to work in every environment.

// The unified server model (the `Server` orchestrator, `ServerBackend` trait,
// `ServerBackends` registry), the `HttpServer` component and its
// `set_http_server` install hook are all no_std-capable and compile
// unconditionally; the concrete backends below stay std/feature-gated.
mod server_backend;
pub use server_backend::*;
mod http_server;
pub use http_server::*;

#[cfg(feature = "std")]
mod cli_server;
#[cfg(feature = "std")]
pub use cli_server::*;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
mod repl_server;
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub use repl_server::*;
#[cfg(all(feature = "server", feature = "json", not(target_arch = "wasm32")))]
mod echo_http_server;
#[cfg(all(feature = "hyper", not(target_arch = "wasm32")))]
mod hyper_server;
#[cfg(all(feature = "lambda", not(target_arch = "wasm32")))]
mod lambda_server;
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod mini_http_server;
#[cfg(feature = "std")]
mod server_plugin;
#[cfg(all(
	feature = "server",
	feature = "json",
	not(target_arch = "wasm32")
))]
pub use echo_http_server::*;
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
#[cfg(feature = "std")]
pub use server_plugin::*;
