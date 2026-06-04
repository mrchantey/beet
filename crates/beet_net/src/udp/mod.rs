//! UDP datagram sockets: the transport seam that protocols like mDNS ride on.
//!
//! UDP is kept deliberately distinct from any protocol spoken over it — mDNS
//! lives in its own [`mdns`](crate::mdns) module. This module has two layers,
//! each independently gated so the trait builds `no_std` and only the std
//! driver pulls `async-io`:
//!
//! - [`udp_socket`]: the cheap, trait-only [`UdpEndpoint`] / [`UdpSocket`]
//!   abstraction. It carries no runtime and no buffers of its own, so it is a
//!   pure handle around whatever the platform's datagram socket is. This is
//!   beet's general socket-level seam, reusable for any UDP protocol (mDNS,
//!   SNTP, plain DNS), not mDNS-specific.
//! - [`impl_async_io`]: a std implementation of that trait over `async-io`,
//!   gated behind the `udp` feature. A `no_std` target supplies its own
//!   implementation of the same trait instead.

mod udp_socket;
pub use udp_socket::*;

// The std `async-io` impl needs `async-io` + `futures-lite`, which only the
// `udp` feature pulls in (mirroring how `server` gates the std TCP backend).
#[cfg(all(feature = "udp", not(target_arch = "wasm32")))]
mod impl_async_io;
#[cfg(all(feature = "udp", not(target_arch = "wasm32")))]
pub use impl_async_io::*;
