//! A cheap, trait-only UDP/datagram socket abstraction.
//!
//! This is beet's own socket seam, mirroring the *method shape* of
//! [`edge-nal`](https://docs.rs/edge-nal)'s `UdpBind` / `UdpReceive` /
//! `UdpSend` / `MulticastV4` traits so that an `edge-nal`-backed adapter (the
//! esp32/embassy-net side) is a thin wrapper, **without** taking `edge-nal` as
//! a public dependency. The rationale is the same as why beet owns
//! [`Request`](crate::prelude::Request) / [`Response`](crate::prelude::Response)
//! instead of re-exporting `http`'s types: beet's API stays beet-owned and is
//! not coupled to edge-net's release cadence or design.
//!
//! The trait carries **no runtime and no buffers of its own** — it is purely a
//! handle around whatever the platform's socket type is. The caller supplies the
//! receive/send buffers, exactly like `edge-nal` and `embassy-net`'s
//! `UdpSocket`. This keeps it `no_std`-compatible (the esp impl lives downstream
//! and supplies non-`Send` embassy futures), while the std impl over `async-io`
//! lives in [`impl_async_io`](super::impl_async_io) behind the `std` feature.
//!
//! Two associated items make up the seam:
//!
//! - [`UdpEndpoint`]: a *binder* — `bind(local) -> Socket`. On std this is a unit
//!   struct; on esp it carries the `embassy-net` stack handle.
//! - [`UdpSocket`]: a bound socket with [`send_to`](UdpSocket::send_to),
//!   [`recv_from`](UdpSocket::recv_from) and
//!   [`join_multicast_v4`](UdpSocket::join_multicast_v4).
//!
//! Addresses use [`core::net`] types so the surface is `no_std`; on std,
//! `std::net::SocketAddr` *is* `core::net::SocketAddr`, so no conversion is
//! needed at the boundary.

use beet_core::prelude::*;
use core::future::Future;
use core::net::Ipv4Addr;
use core::net::SocketAddr;

/// A binder for UDP sockets: opens a [`UdpSocket`] bound to a local address.
///
/// Mirrors `edge-nal`'s `UdpBind`. The endpoint owns whatever per-platform
/// context is needed to open a socket (nothing on std; the platform network-stack
/// handle on esp), so a [`UdpSocket`] can be opened from it on demand.
///
/// This is also the natural shape for a config component: a downstream driver
/// reads the `UdpEndpoint` off an entity and calls [`bind`](UdpEndpoint::bind)
/// to obtain the live socket.
pub trait UdpEndpoint {
	/// The socket type produced by [`bind`](Self::bind).
	type Socket<'a>: UdpSocket
	where
		Self: 'a;

	/// Bind a new UDP socket to `local`.
	///
	/// Pass port `0` to let the OS choose an ephemeral port. To receive
	/// multicast (e.g. mDNS) the caller typically binds the multicast port on
	/// `0.0.0.0` and then calls
	/// [`join_multicast_v4`](UdpSocket::join_multicast_v4).
	fn bind(
		&self,
		local: SocketAddr,
	) -> impl Future<Output = Result<Self::Socket<'_>>>;
}

/// A bound UDP socket: send datagrams to peers, receive datagrams from peers,
/// and join IPv4 multicast groups.
///
/// Mirrors `edge-nal`'s `UdpReceive` + `UdpSend` + `MulticastV4`, collapsed onto
/// one trait because beet always wants all three together (an mDNS browser both
/// sends queries and receives answers on the same multicast socket). The methods
/// are buffer-in/buffer-out with no internal allocation, so the impl is a thin
/// handle.
///
/// Note there is intentionally **no** client/server split: UDP is connectionless,
/// so a single `UdpSocket` both sends and receives. (This is why the type is
/// `UdpSocket`, not `UdpClient`.)
pub trait UdpSocket {
	/// Send `bytes` as a single datagram to `remote`.
	///
	/// Returns once the datagram has been handed to the network stack.
	fn send_to(
		&self,
		bytes: &[u8],
		remote: SocketAddr,
	) -> impl Future<Output = Result<()>>;

	/// Receive a single datagram into `buf`, returning the number of bytes
	/// written and the sender's address.
	///
	/// `buf` should be large enough for the largest expected datagram; a
	/// datagram longer than `buf` may be truncated (per the platform's UDP
	/// semantics).
	fn recv_from(
		&self,
		buf: &mut [u8],
	) -> impl Future<Output = Result<(usize, SocketAddr)>>;

	/// Join the IPv4 multicast group `group` so this socket receives datagrams
	/// sent to it (e.g. `224.0.0.251` for mDNS).
	fn join_multicast_v4(
		&self,
		group: Ipv4Addr,
	) -> impl Future<Output = Result<()>>;
}
