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
//!   [`run_mdns_browser`] (the std socket driver) needs `std`.
//!
//! The platform-specific piece (binding the socket, joining multicast, sending
//! the periodic query) is supplied by the caller via the [`udp`](crate::udp)
//! traits — on std by [`run_mdns_browser`], on esp by a downstream embassy loop
//! that bridges datagrams into [`UdpPacket`].

use core::net::Ipv4Addr;
use core::net::SocketAddr;
use core::net::SocketAddrV4;

pub mod wire;
pub use wire::*;

mod browser;
pub use browser::*;

/// The IPv4 mDNS multicast group, `224.0.0.251`.
pub const MDNS_MULTICAST_V4: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// The mDNS UDP port, `5353`.
pub const MDNS_PORT: u16 = 5353;

/// The standard mDNS multicast socket address, `224.0.0.251:5353`.
///
/// Sent *to* by queriers and responders; the socket itself is bound on
/// `0.0.0.0:5353` and joined to [`MDNS_MULTICAST_V4`] to receive it.
pub const MDNS_ENDPOINT: SocketAddr =
	SocketAddr::V4(SocketAddrV4::new(MDNS_MULTICAST_V4, MDNS_PORT));
