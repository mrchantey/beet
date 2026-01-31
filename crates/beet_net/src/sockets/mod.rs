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
//! let mut socket = Socket::connect("wss://echo.websocket.org").await?;
//! socket.send(Message::text("hello")).await?;
//! # Ok(())
//! # }
//! ```
pub mod common_handlers;
mod socket;
pub use socket::Message;
pub use socket::*;
mod socket_server;
pub use socket_server::*;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
mod impl_tungstenite;
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
pub(crate) use impl_tungstenite::start_tungstenite_server;
#[cfg(target_arch = "wasm32")]
pub(self) mod impl_web_sys;
