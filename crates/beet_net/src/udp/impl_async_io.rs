//! The std implementation of [`UdpEndpoint`] / [`UdpSocket`] over `async-io`.
//!
//! `async-io` is already a `beet_net` dependency for the `server` feature, so
//! the std side needs no edge-net crates: we drive a plain
//! [`std::net::UdpSocket`] through [`async_io::Async`]. Multicast join goes
//! through `std::net::UdpSocket::join_multicast_v4` on the underlying socket
//! (reachable via [`Async::get_ref`](async_io::Async::get_ref)).
//!
//! This is the impl the agnostic mDNS browser engine is exercised against on
//! std, where the socket runtime and the world runtime coincide so no
//! embassy-to-world bridge is needed (see [`super::browser`]).

use super::UdpEndpoint;
use super::UdpSocket;
use async_io::Async;
use beet_core::prelude::*;
use core::net::Ipv4Addr;
use core::net::SocketAddr;

/// std [`UdpEndpoint`] binding sockets via `async-io`.
///
/// Carries no state: on std there is no per-platform context to thread through,
/// so binding just opens an [`Async<std::net::UdpSocket>`](async_io::Async).
#[derive(Debug, Default, Clone, Copy)]
pub struct AsyncIoUdpEndpoint;

impl UdpEndpoint for AsyncIoUdpEndpoint {
	type Socket<'a> = AsyncIoUdpSocket;

	async fn bind(&self, local: SocketAddr) -> Result<Self::Socket<'_>> {
		// `reuse_address` so the mDNS port can be shared with the OS daemon /
		// other listeners, matching responder/browser coexistence.
		let socket = std::net::UdpSocket::bind(local)
			.map_err(|err| bevyhow!("UDP bind {local} failed: {err}"))?;
		let socket = Async::new(socket)
			.map_err(|err| bevyhow!("UDP async wrap failed: {err}"))?;
		Ok(AsyncIoUdpSocket { socket })
	}
}

/// A std [`UdpSocket`] backed by [`async_io::Async`].
pub struct AsyncIoUdpSocket {
	socket: Async<std::net::UdpSocket>,
}

impl AsyncIoUdpSocket {
	/// The local address this socket is bound to.
	pub fn local_addr(&self) -> Result<SocketAddr> {
		self.socket
			.get_ref()
			.local_addr()
			.map_err(|err| bevyhow!("UDP local_addr failed: {err}"))
	}
}

impl UdpSocket for AsyncIoUdpSocket {
	async fn send_to(&self, bytes: &[u8], remote: SocketAddr) -> Result<()> {
		self.socket
			.send_to(bytes, remote)
			.await
			.map_err(|err| bevyhow!("UDP send_to {remote} failed: {err}"))?;
		Ok(())
	}

	async fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
		self.socket
			.recv_from(buf)
			.await
			.map_err(|err| bevyhow!("UDP recv_from failed: {err}"))
	}

	async fn join_multicast_v4(&self, group: Ipv4Addr) -> Result<()> {
		// Join on the unspecified interface; the OS picks the right one. Joining
		// the same group twice is harmless / idempotent for our purposes, so an
		// "already joined" error is swallowed.
		match self
			.socket
			.get_ref()
			.join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED)
		{
			Ok(()) => Ok(()),
			Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => Ok(()),
			Err(err) => {
				Err(bevyhow!("UDP join_multicast_v4 {group} failed: {err}"))
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::net::Ipv4Addr;
	use core::net::SocketAddr;
	use core::net::SocketAddrV4;

	#[beet_core::test]
	async fn loopback_roundtrip() {
		let endpoint = AsyncIoUdpEndpoint;
		let loopback = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0));

		let server = endpoint.bind(loopback).await.unwrap();
		let server_addr = server.local_addr().unwrap();

		let client = endpoint.bind(loopback).await.unwrap();
		client.send_to(b"ping", server_addr).await.unwrap();

		let mut buf = [0u8; 64];
		let (n, from) = server.recv_from(&mut buf).await.unwrap();
		(&buf[..n]).xpect_eq(b"ping".as_slice());
		from.ip().is_loopback().xpect_true();
	}

	#[beet_core::test]
	async fn join_multicast_is_ok() {
		let endpoint = AsyncIoUdpEndpoint;
		let bind = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
		let socket = endpoint.bind(bind).await.unwrap();
		// joining a multicast group (the mDNS group `224.0.0.251`) must not error
		// on a normal host. Spelled out here so the UDP layer's tests stay free of
		// any mDNS dependency.
		socket
			.join_multicast_v4(Ipv4Addr::new(224, 0, 0, 251))
			.await
			.xpect_ok();
	}
}
