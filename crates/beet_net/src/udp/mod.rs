//! UDP/datagram sockets and the agnostic mDNS service browser.
//!
//! This module has three layers, each independently gated so the trait builds
//! `no_std` and only the std driver pulls `async-io`:
//!
//! - [`udp_socket`]: the cheap, trait-only [`UdpEndpoint`] / [`UdpSocket`]
//!   abstraction (mirrors `edge-nal`'s method shape, not its types). `no_std`,
//!   always compiled — this is beet's first socket-level seam, reusable beyond
//!   mDNS (SNTP, plain DNS). See decision 4 in `agent/plans/mdns.md`.
//! - [`impl_async_io`]: the std impl over `async-io`. `std`-gated; esp supplies
//!   its own `edge-nal`-backed impl downstream.
//! - [`mdns`] + [`browser`]: the agnostic mDNS service browser — pure wire
//!   helpers plus a bytes-and-world ECS engine (events + a resource, not
//!   actions). Gated behind the `mdns` feature; the parse + engine are `no_std`,
//!   only [`run_mdns_browser`] (the std socket driver) needs `std`.

mod udp_socket;
pub use udp_socket::*;

// The std `async-io` impl needs `async-io` + `futures-lite`, which only the
// `udp` feature pulls in (mirroring how `server` gates the std TCP backend).
#[cfg(all(feature = "udp", not(target_arch = "wasm32")))]
mod impl_async_io;
#[cfg(all(feature = "udp", not(target_arch = "wasm32")))]
pub use impl_async_io::*;

#[cfg(feature = "mdns")]
pub mod mdns;
#[cfg(feature = "mdns")]
mod browser;
#[cfg(feature = "mdns")]
pub use browser::*;
