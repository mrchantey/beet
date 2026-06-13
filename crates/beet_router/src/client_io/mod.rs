//! A general purpose server-to-client websocket channel ([`ClientIo`]) and the
//! dev-mode live reload built on it ([`LiveReload`], [`LiveReloadScript`]).
//!
//! The channel rides the main HTTP port via the same-port websocket upgrade
//! seam: [`client_io_route`] (wired into `default_router`) serves a
//! [`WebSocketUpgrade`](beet_net::prelude::WebSocketUpgrade) at
//! [`CLIENT_IO_PATH`], the backend lands the upgraded connection as a
//! [`Socket`](beet_net::prelude::Socket) entity, and [`adopt_client_io_socket`]
//! re-parents it under the channel. Connected browsers are thus child `Socket`
//! entities; [`ClientIoBroadcast`] fans a message out to all of them.

mod client_io;
pub use client_io::*;
mod live_reload;
pub use live_reload::*;
mod live_reload_script;
pub use live_reload_script::*;
