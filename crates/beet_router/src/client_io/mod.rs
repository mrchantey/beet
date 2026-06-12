//! A general purpose server-to-client websocket channel ([`ClientIo`]) and the
//! dev-mode live reload built on it ([`LiveReload`], [`LiveReloadScript`]).
//!
//! None of the HTTP server backends (`mini_http`, `hyper`, `lambda`) support
//! websocket upgrades, so the channel runs a [`SocketServer`](beet_net::prelude::SocketServer)
//! on its own port beside the HTTP server. Connected browsers are child
//! [`Socket`](beet_net::prelude::Socket) entities; [`ClientIoBroadcast`] fans a
//! message out to all of them.

mod client_io;
pub use client_io::*;
mod live_reload;
pub use live_reload::*;
mod live_reload_script;
pub use live_reload_script::*;
