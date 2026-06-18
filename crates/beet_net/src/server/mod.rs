//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Action<Request, Response>`.
//!
//! ## Every binary is a CLI server
//!
//! A formal beet binary boots as a CLI server at the top level: its entrypoint
//! spawns a [`CliServer`] host and triggers [`StartServer::all`] on it, parsing
//! argv into a [`Request`] and running one exchange.
//! Long-running servers ([`HttpServer`], the `beet_router` `TuiServer`) are
//! started the same way, by a [`StartServer`] event whose filter selects them.
//! See [`server_events`] for the model.
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

// The `HttpServer` component and its `set_http_server` install hook are
// no_std-capable and compile unconditionally; the concrete backends below stay
// std/feature-gated.
mod http_server;
pub use http_server::*;
// The server model events (`StartServer` / `StopServer`) and the `KeepAlive`
// resource are no_std (`GlobFilter` and `MultiMap` both build no_std), so a
// no_std target boots `HttpServer` through the very same `StartServer` observers,
// supplying its backend via `set_http_server`. Only the async-runtime dispatch
// (`queue_async_local`) inside the observers stays std-gated.
mod server_events;
pub use server_events::*;

#[cfg(feature = "std")]
mod cli_server;
#[cfg(feature = "std")]
pub use cli_server::*;
// The `ServeOnLoad` markup verb that boots a host's declared servers on
// `LoadTemplate`, by triggering a `StartServer` built from the entry request.
#[cfg(feature = "std")]
mod serve_on_load;
#[cfg(feature = "std")]
pub use serve_on_load::*;
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
