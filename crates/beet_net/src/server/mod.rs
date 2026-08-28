//! HTTP server implementations for handling incoming requests.
//!
//! This module provides server infrastructure that listens for HTTP requests
//! and routes them to Bevy entities for processing via `Action<Request, Response>`.
//!
//! ## One load path, one parked action
//!
//! [`CallOnReady`] calls its entity's action with the process request. On a server
//! entity that action belongs to the [`RunningSet`] its servers added facets to:
//! it inserts a `Running<Response>` keep-alive claim, fires a
//! `StartRunning<Request>` for observers, then drives every facet the start
//! selected. A one-shot [`CliServer`] resolves the call (its response streams to
//! stdout and the process exits); a long-running [`HttpServer`] / `TuiServer`
//! holds its facet open, persisting the process until that `Running` is removed,
//! which signals every live facet. `--server` selects which servers act (see
//! [`RunningSetFilter`]), and a selection matching none fails the call rather
//! than parking. The dispatch host (a `Router`) is a child, reached with
//! `exchange_child`. See [`call_on_ready`] and
//! [`RunningSet`](beet_action::prelude::RunningSet) for the model.
//!
//! ## Implementations
//!
//! - **Mini HTTP**: Lightweight async-io TCP server (default for `server` feature)
//! - **Hyper**: Full-featured HTTP server (requires `hyper` feature)
//! - **Lambda**: AWS Lambda runtime adapter (requires `lambda` feature)
//! - **Installed**: a backend supplied at runtime via [`HttpServer::set_backend`], used
//!   on `no_std` targets with no compiled-in backend.
//!
//! The server backend is selected at compile time based on feature flags.
//! All implementations route requests through the action-based exchange
//! pattern, allowing the same handler code to work in every environment.

// The process-global loopback port. Action-free and unconditional, unlike
// everything else here: a client rewriting an authority-less request reads it in
// a build that compiled no server at all.
mod canonical_port;
pub use canonical_port::*;

// The `HttpServer` component and its `HttpServer::set_backend` install hook; the concrete
// backends below stay std/feature-gated on top. A server adds one facet to its
// entity's `RunningSet`, signalled when that set's parked `Running<Response>` is
// removed.
//
// Gated on `action`, not `std`: a server dispatches through an
// `Action<Request, Response>` by construction, so there is no server without the
// action layer. `action` is itself no_std-capable, so an embedded host still gets
// the whole lifecycle.
#[cfg(feature = "action")]
mod http_server;
#[cfg(feature = "action")]
pub use http_server::*;

// The bind knobs a booting server reads off its start request. Action-free: it
// is a plain params type over a `Request`.
mod server_params;
pub use server_params::*;

// In-memory channel-backed HTTP server: shares `HttpServer`'s boot/park/dispatch
// machinery over an `async_channel` instead of a socket. `std` for `async-channel`,
// but deliberately not wasm-gated (the wasm-runnable server path).
#[cfg(feature = "std")]
mod channel_http_server;
#[cfg(feature = "std")]
pub use channel_http_server::*;

// The load path: the `CallOnReady` verb calls an entity's action with the
// process request and writes `AppExit`. Gated on `action` (the call machinery),
// not `std`: `CliArgs::parse_env` no-ops on no_std, the stdout tail goes through
// the cross-platform `cross_log_noline!`, and the verb / `AppExit` writer are
// all no_std-clean, so an embedded boot works too.
#[cfg(feature = "action")]
mod call_on_ready;
#[cfg(feature = "action")]
pub use call_on_ready::*;

// The start path: the `CallOnStart` verb calls an entity's action when the run
// above it starts, observing its own entity's swept `StartRunning<Request>`.
// Gated on `action` like the load verb it mirrors.
#[cfg(feature = "action")]
mod call_on_start;
#[cfg(feature = "action")]
pub use call_on_start::*;

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
#[cfg(feature = "std")]
pub use server_plugin::*;
