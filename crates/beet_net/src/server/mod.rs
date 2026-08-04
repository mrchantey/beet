//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Action<Request, Response>`.
//!
//! ## One load path, servers own the boot
//!
//! [`CallOnLoad`] calls its entity's action with the process request. On a
//! server that action is [`StartOnLoad`]'s: it inserts a `Running<Response>`
//! keep-alive claim and fires an `StartRunning<Request>` every server on the
//! entity observes. A one-shot [`CliServer`] resolves the call (its response
//! streams to stdout and the process exits); a long-running [`HttpServer`] /
//! `TuiServer` parks the call, persisting the process until its `Running` is
//! removed, which fires its teardown observer. `--server` selects which servers
//! act, and a selection matching none exits rather than parking. The dispatch
//! host (a `Router`) is a child, reached with `exchange_child`. See
//! [`call_on_load`] and [`start_on_load`] for the model.
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
// std/feature-gated. A server is an `StartRunning<Request>` observer torn down
// by observing the removal of `StartOnLoad`'s parked `Running<Response>`.
mod http_server;
pub use http_server::*;

// The shared boot, park and shutdown lifecycle every bootable server uses, keyed by
// the server marker. no_std-capable like `HttpServer`, which depends on it.
mod server_lifecycle;
pub use server_lifecycle::*;

// In-memory channel-backed HTTP server: shares `HttpServer`'s boot/park/dispatch
// machinery over an `async_channel` instead of a socket. `std` for `async-channel`,
// but deliberately not wasm-gated (the wasm-runnable server path).
#[cfg(feature = "std")]
mod channel_http_server;
#[cfg(feature = "std")]
pub use channel_http_server::*;

// The load path: the `CallOnLoad` verb calls an entity's action with the
// process request and writes `AppExit`. Gated on `action` (the call machinery),
// not `std`: `CliArgs::parse_env` no-ops on no_std, the stdout tail goes through
// the cross-platform `cross_log_noline!`, and the verb / `AppExit` writer are
// all no_std-clean, so an embedded boot works too.
#[cfg(feature = "action")]
mod call_on_load;
#[cfg(feature = "action")]
pub use call_on_load::*;

// The boot verb: parks the process and fans the request out to the servers on
// its entity. Same `action` gate as the load verb it requires.
#[cfg(feature = "action")]
mod start_on_load;
#[cfg(feature = "action")]
pub use start_on_load::*;

#[cfg(feature = "action")]
mod cli_server;
#[cfg(feature = "action")]
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
// Classify accepted connections by their first bytes (TLS ClientHello vs
// plaintext http) with replay: the seam that lets a `Tls` listener keep
// serving plaintext peers, and any socket listener answer a browser `GET`.
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
pub mod stream_sniff;
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
