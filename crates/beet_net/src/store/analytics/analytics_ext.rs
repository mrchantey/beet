//! Free helpers shared across the analytics emitters: reading the session and
//! client address from request headers, and the current server time.
use crate::prelude::*;
use beet_core::prelude::*;

/// The session cookie the web client sets; read from a request to attribute it
/// to a session (the same id the client sends in its page-view beacons).
pub const SESSION_COOKIE: &str = "beet_session";

/// Milliseconds since the unix epoch, cross-platform via [`time_ext`].
pub fn now_ms() -> u64 { time_ext::now_millis() as u64 }

/// The client ip from request headers: `cf-connecting-ip` when present (set by
/// a fronting Cloudflare proxy and not client-spoofable, unlike the first
/// `x-forwarded-for` hop), then a proxy's `x-forwarded-for` (its client-most
/// hop), then the direct peer address the server tagged as
/// [`PEER_ADDR_HEADER`]. `None` when none are present or parseable.
pub fn client_ip(headers: &HeaderMap) -> Option<std::net::IpAddr> {
	if let Some(connecting) = headers.first_raw("cf-connecting-ip") {
		connecting.trim().parse().ok()
	} else if let Some(forwarded) = headers.first_raw("x-forwarded-for") {
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

#[cfg(test)]
mod test {
	use super::*;

	/// Proxy headers win over the peer address, `cf-connecting-ip` first.
	#[beet_core::test]
	fn client_ip_precedence() {
		let mut headers = HeaderMap::default();
		headers.set_raw(PEER_ADDR_HEADER, "10.0.0.1:443");
		client_ip(&headers)
			.unwrap()
			.to_string()
			.as_str()
			.xpect_eq("10.0.0.1");
		// a proxy's client-most forwarded hop beats the peer address
		headers.set_raw("x-forwarded-for", "203.0.113.7, 10.0.0.1");
		client_ip(&headers)
			.unwrap()
			.to_string()
			.as_str()
			.xpect_eq("203.0.113.7");
		// the cloudflare-set header beats the (client-spoofable) forwarded hop
		headers.set_raw("cf-connecting-ip", "198.51.100.4");
		client_ip(&headers)
			.unwrap()
			.to_string()
			.as_str()
			.xpect_eq("198.51.100.4");
	}
}
