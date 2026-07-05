//! TLS serving for the native listeners: a [`Tls`] component beside
//! [`HttpServer`](crate::prelude::HttpServer) /
//! [`SocketServer`](crate::sockets::SocketServer) wraps their accept loops in
//! a rustls acceptor.
//!
//! The point is the browser secure context: `navigator.mediaDevices`, and
//! most powerful web APIs, exist only on https/wss or localhost origins, so a
//! phone on the LAN opening `http://192.168.x.x` cannot use the webcam. With
//! [`Tls`] the same servers answer `https`/`wss` after a one-time self-signed
//! certificate warning, on every device and platform.
//!
//! Listeners keep sniffing each connection
//! ([`SecureProtocol::sniff`](crate::prelude::stream_sniff::SecureProtocol)),
//! so serving TLS is additive: plaintext loopback (the reload watcher, `curl`,
//! `localhost` tabs) and plaintext native websocket peers (an esp body) keep
//! working, and a remote browser typing `http://` is redirected to `https`.
//!
//! Certificates come from the cached self-signed [`DevCert`] by default, or
//! from provided PEM paths for real certificates. Managed-platform detection
//! keeps the component inert where a platform layer terminates TLS instead
//! (an ALB in front of Fargate, a lambda gateway); `BEET_TLS=on`/`off`
//! overrides it.

mod tls;
pub use tls::*;
#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
mod dev_cert;
#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
pub use dev_cert::*;
#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
mod server_tls;
#[cfg(all(feature = "secure", not(target_arch = "wasm32")))]
pub use server_tls::*;
// raw clients trusting the dev cert, shared by the mini/tungstenite tls tests
#[cfg(all(test, feature = "secure", not(target_arch = "wasm32")))]
pub(crate) mod test_client;
