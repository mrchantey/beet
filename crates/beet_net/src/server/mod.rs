//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Action<Request, Response>`.
//!
//! ## One boot path, servers are observers
//!
//! [`boot`] calls a host's `Action<Boot, Response>` slot with `Boot(request)`:
//! the server-provided `ContinueRun<Boot, Response>` inserts a `Running<Response>`
//! keep-alive claim and fires an `ActionIn<Boot>` the servers observe. A one-shot
//! [`CliServer`] resolves the call (its response streams to stdout and the process
//! exits); a long-running [`HttpServer`] / `TuiServer` parks the call, persisting
//! the process until its `Running` is removed, which fires its teardown observer.
//! `--server` selects which servers act. See [`boot`] for the model.
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
// std/feature-gated. A server is an `ActionIn<Boot>` observer torn down by
// observing the removal of the boot exchange's `Running<Response>`.
mod http_server;
pub use http_server::*;

// The boot path: the `BootOnLoad` / `ExchangeOnLoad` verbs call a host's action
// slot with the process request and write `AppExit`. std-only (it reads CLI args,
// streams to stdout, and writes the exit).
#[cfg(feature = "std")]
mod boot;
#[cfg(feature = "std")]
pub use boot::*;

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
