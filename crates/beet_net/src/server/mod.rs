//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Action<Request, Response>`.
//!
//! ## One boot path, servers are observers
//!
//! [`boot`] calls a host's `Action<Boot, Response>` slot with `Boot(request)`:
//! the server-provided `ContinueRun<Boot, Response>` inserts a `Running<Response>`
//! keep-alive claim and fires an `StartRunning<Boot>` the servers observe. A one-shot
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
// std/feature-gated. A server is an `StartRunning<Boot>` observer torn down by
// observing the removal of the boot exchange's `Running<Response>`.
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

// The no_std core of the boot path: the `Boot` exchange newtype and the
// `request_selects_server` predicate the unconditionally-compiled `HttpServer`
// observer needs. Compiles everywhere; the std-only verbs that drive it are in
// `boot` below.
mod boot_exchange;
pub use boot_exchange::*;

// The boot path: the `BootOnLoad` / `ExchangeOnLoad` verbs call a host's action
// slot with the process request and write `AppExit`. Gated on `action` (the
// `Action<Boot, Response>` slot), not `std`: `CliArgs::parse_env` no-ops on no_std,
// the stdout tail goes through the cross-platform `cross_log_noline!`, and the boot
// verbs / `AppExit` writer are all no_std-clean, so an embedded boot works too.
#[cfg(feature = "action")]
mod boot;
#[cfg(feature = "action")]
pub use boot::*;

// the boot<->exchange bridges, inverses on the same host: a boot drives its request
// pipeline (`BootToExchange`) or a request drives its boot (`ExchangeToBoot`).
// no_std-clean like the boot path, gated on the `Action` slots they install.
#[cfg(feature = "action")]
mod boot_bridge;
#[cfg(feature = "action")]
pub use boot_bridge::*;

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
