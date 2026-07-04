//! WebSocket client and server implementations.
//!
//! This module provides cross-platform WebSocket support:
//!
//! - **Client**: [`Socket::connect`] for establishing connections
//! - **Server**: [`SocketServer`] for accepting incoming connections
//!
//! ## Platform Support
//!
//! - **WASM**: Uses `web-sys` WebSocket API
//! - **Native**: Uses `tungstenite` (requires `tungstenite` feature)
//!
//! ## Example
//!
//! ```ignore
//! # use beet_net::sockets::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let mut socket = Socket::connect("ws://127.0.0.1:8339").await?;
//! socket.send(Message::text("hello")).await?;
//! # Ok(())
//! # }
//! ```
/// Common WebSocket message handlers for ping/pong and other utilities.
pub mod common_handlers;
#[cfg(all(test, feature = "tungstenite", not(target_arch = "wasm32")))]
mod echo_socket_server;
mod socket;
pub use socket::Message;
pub use socket::*;
/// The agnostic no_std mpsc used by the socket internals (`Socket::effect`'s
/// writer feed, [`SocketClosedNotify`]); public so consumers can build the
/// channel ends those components carry.
pub mod writer_channel;
// The self-reconnecting client socket. Its backoff sleeps ride the std-gated
// `time_ext::sleep` (available on wasm too); a bare-metal transport reconnects
// at its own driver layer instead.
#[cfg(feature = "std")]
mod persistent_socket;
#[cfg(feature = "std")]
pub use persistent_socket::*;
// The server backend boots through `HttpServer`'s installable-backend model and
// its `new_test` binds real TCP: std-only. The no_std client core does not need
// it, so it rides `std` (the bare-metal device is a client).
#[cfg(feature = "std")]
mod socket_server;
#[cfg(feature = "std")]
pub use socket_server::*;
// In-memory channel-backed WebSocket server: the socket analogue of
// `ChannelHttpServer`. Built on `async_channel` (std-only), so gated on `std`.
#[cfg(feature = "std")]
mod channel_socket_server;
#[cfg(feature = "std")]
pub use channel_socket_server::*;
// The Request/Response exchange carried over a socket; needs the `RequestParts` /
// `ResponseParts` serde derives the frame serializes, plus the `action` layer
// that `sockets` already pulls. It is no_std-capable, so it rides `serde` +
// `sockets` and reaches the bare-metal device build (the esp body serves
// `apply-heading` over it), not just `std` (which turns `sockets` on anyway).
#[cfg(all(feature = "serde", feature = "sockets"))]
mod socket_exchange;
#[cfg(all(feature = "serde", feature = "sockets"))]
pub use socket_exchange::*;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
mod impl_tungstenite;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub(crate) use impl_tungstenite::socket_from_upgraded;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub use impl_tungstenite::start_tungstenite_server;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub(crate) use impl_tungstenite::start_tungstenite_server_with_tcp;
#[cfg(target_arch = "wasm32")]
pub(self) mod impl_web_sys;
