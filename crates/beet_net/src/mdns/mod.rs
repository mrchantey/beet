//! mDNS (`.local` name service / DNS-SD) over UDP multicast.
//!
//! mDNS is the DNS wire format (RFC 1035) carried over UDP multicast on
//! `224.0.0.251:5353`. It is a distinct protocol that *uses* the [`udp`](crate::udp)
//! socket seam; the two are kept separate (UDP is the transport, mDNS is one
//! protocol spoken over it). This module has two layers, each independently
//! gated so the wire codec builds `no_std` and only the std driver pulls
//! `async-io`:
//!
//! - [`wire`]: pure mDNS wire helpers — build a `PTR` query, parse a response
//!   into [`Record`]s. No sockets, no world, `no_std`, always compiled with the
//!   `mdns` feature.
//! - [`browser`]: the agnostic service **browser** engine — a bytes-and-world
//!   ECS layer that turns inbound datagrams into one entity per discovered
//!   service ([`MDnsService`]). The parse + engine are `no_std`; only
//!   [`MdnsBrowser::run`] (the std socket driver) needs `std`.
//!
//! The platform-specific piece (binding the socket, joining multicast, sending
//! the periodic query) is supplied by the caller via the [`udp`](crate::udp)
//! traits — on std by [`MdnsBrowser::run`], on esp by a downstream embassy loop
//! that bridges datagrams into [`UdpPacket`].

pub mod wire;
pub use wire::MdnsResponse;
pub use wire::Record;

mod browser;
pub use browser::*;

