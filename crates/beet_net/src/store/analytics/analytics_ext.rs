//! Free helpers shared across the analytics emitters: reading the session and
//! client address from request headers, and the current server time.
use crate::prelude::*;
use beet_core::prelude::*;

/// The session cookie the web client sets; read from a request to attribute it
/// to a session (the same id the client sends in its page-view beacons).
pub const SESSION_COOKIE: &str = "beet_session";

/// Milliseconds since the unix epoch, cross-platform via [`time_ext`].
pub fn now_ms() -> u64 { time_ext::now_millis() as u64 }

/// The client ip from request headers: a proxy's `x-forwarded-for` (its
/// client-most hop) when present, else the direct peer address the server tagged
/// as [`PEER_ADDR_HEADER`]. `None` when neither is present or parseable.
pub fn client_ip(headers: &HeaderMap) -> Option<std::net::IpAddr> {
	if let Some(forwarded) = headers.first_raw("x-forwarded-for") {
		forwarded.split(',').next()?.trim().parse().ok()
	} else {
		headers
			.first_raw(PEER_ADDR_HEADER)
			.and_then(|peer| peer.parse::<std::net::SocketAddr>().ok())
			.map(|peer| peer.ip())
	}
}

/// The [`SESSION_COOKIE`] value parsed as a session id, searching every `Cookie`
/// header line's `key=value` pairs.
pub fn session_from_cookies(headers: &HeaderMap) -> Option<Uuid> {
	headers
		.get::<header::Cookie>()
		.and_then(|res| res.ok())?
		.iter()
		.flat_map(|line| line.split(';'))
		.filter_map(|pair| pair.trim().split_once('='))
		.find(|(key, _)| *key == SESSION_COOKIE)
		.and_then(|(_, value)| value.parse().ok())
}
