//! HTTP parsing and wire utilities.
//!
//! This module is split into three groups:
//!
//! - **`no_std` wire helpers** ([`find_header_end`], [`parse_content_length`],
//!   [`parse_http_request`], [`serialize_http_response`]): operate only on byte
//!   buffers and beet's [`Request`]/[`Response`] wire types, so they are
//!   `no_std` and shared by the std `mini_http_server` and any downstream
//!   embedded backend (eg an esp/embassy WiFi server) installed via
//!   [`set_http_server`]. Only the *pure* parse/serialise half is shareable; the
//!   connection + streaming half differs per transport (std `async-io` vs an
//!   embassy `TcpSocket`) and stays in each backend.
//! - **websocket upgrade helpers**: the transport-agnostic, mostly-pure
//!   handshake seam. [`is_websocket_upgrade`]/[`is_websocket_response`] are
//!   `no_std`; the accept-key digest (`sec_websocket_accept`) and the
//!   `WebSocketUpgrade` response type a route returns are gated behind
//!   `tungstenite` (they pull `sha1`/`base64`). A backend lands the upgraded
//!   stream as a `Socket` (see `mini_http_server`/`hyper_server`).
//! - **`http`-crate helpers** ([`has_body`], [`version_to_string`],
//!   [`parse_version`]): convert to/from the `http` crate types and are gated
//!   per-function behind the `http` feature, so the module itself still compiles
//!   `no_std` without that feature.

use crate::prelude::*;
use beet_core::prelude::*;
use core::fmt::Write;

/// Find the byte offset just past the blank line ending the HTTP headers.
///
/// Returns the position immediately after `\r\n\r\n` (or a lenient `\n\n`), or
/// `None` if the header block has not arrived yet.
pub fn find_header_end(buf: &[u8]) -> Option<usize> {
	if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
		return Some(pos + 4);
	}
	if let Some(pos) = buf.windows(2).position(|window| window == b"\n\n") {
		return Some(pos + 2);
	}
	None
}

/// Extract the `Content-Length` value from raw header bytes (0 if absent).
pub fn parse_content_length(header_bytes: &[u8]) -> usize {
	let header_str = String::from_utf8_lossy(header_bytes);
	for line in header_str.lines() {
		if let Some((key, value)) = line.split_once(':') {
			if key.trim().eq_ignore_ascii_case("content-length") {
				return value.trim().parse::<usize>().unwrap_or(0);
			}
		}
	}
	0
}

/// Parse raw HTTP/1.1 request bytes into a beet [`Request`].
pub fn parse_http_request(raw: &[u8]) -> Result<Request> {
	let raw_str = String::from_utf8_lossy(raw);

	// Split headers from body
	let (header_section, body_bytes) =
		if let Some(split_pos) = raw_str.find("\r\n\r\n") {
			let body_start = split_pos + 4;
			(&raw_str[..split_pos], &raw[body_start..])
		} else if let Some(split_pos) = raw_str.find("\n\n") {
			let body_start = split_pos + 2;
			(&raw_str[..split_pos], &raw[body_start..])
		} else {
			(raw_str.as_ref(), &[][..])
		};

	let mut lines = header_section.lines();

	// Parse request line: METHOD PATH HTTP/VERSION
	let request_line = lines.next().ok_or_else(|| bevyhow!("empty request"))?;
	let mut parts_iter = request_line.split_whitespace();
	let method_str = parts_iter
		.next()
		.ok_or_else(|| bevyhow!("missing HTTP method"))?;
	let path = parts_iter
		.next()
		.ok_or_else(|| bevyhow!("missing HTTP path"))?;

	let method = match method_str.to_ascii_uppercase().as_str() {
		"GET" => HttpMethod::Get,
		"POST" => HttpMethod::Post,
		"PUT" => HttpMethod::Put,
		"DELETE" => HttpMethod::Delete,
		"PATCH" => HttpMethod::Patch,
		"HEAD" => HttpMethod::Head,
		"OPTIONS" => HttpMethod::Options,
		other => {
			return Err(bevyhow!("unsupported HTTP method: {other}"));
		}
	};

	let mut request = Request::new(method, path);

	// Parse headers
	for line in lines {
		if line.is_empty() {
			break;
		}
		if let Some((key, value)) = line.split_once(':') {
			request
				.headers
				.set_raw(key.trim(), value.trim().to_string());
		}
	}

	// Set body if present
	if !body_bytes.is_empty() {
		request.set_body(body_bytes);
	}

	Ok(request)
}

/// The uppercase HTTP token for a method (the [`HttpMethod`] `Display` is
/// title-case, e.g. `Get`, so it can't be written on the wire directly).
fn method_token(method: &HttpMethod) -> &'static str {
	match method {
		HttpMethod::Get => "GET",
		HttpMethod::Post => "POST",
		HttpMethod::Put => "PUT",
		HttpMethod::Patch => "PATCH",
		HttpMethod::Delete => "DELETE",
		HttpMethod::Options => "OPTIONS",
		HttpMethod::Head => "HEAD",
		HttpMethod::Trace => "TRACE",
		HttpMethod::Connect => "CONNECT",
	}
}

/// Headers [`encode_request`] sets itself; user-supplied copies are skipped to
/// avoid duplicates.
fn is_managed_header(key: &str) -> bool {
	key.eq_ignore_ascii_case("host")
		|| key.eq_ignore_ascii_case("content-length")
		|| key.eq_ignore_ascii_case("connection")
}

/// Serialize a beet [`Request`] into raw HTTP/1.1 bytes (origin-form target).
///
/// The inverse of [`parse_http_request`]: writes the request line, a `Host`
/// header from the request authority, the user headers (minus the ones this
/// encoder manages), a computed `content-length`, a `connection: close`, and the
/// body. Returns an error for a [`Body::Stream`], which can't be buffered here;
/// drain it to a [`Body::Bytes`] first if you need to send a stream.
pub fn encode_request(request: &Request) -> Result<Vec<u8>> {
	let body = match &request.body {
		Body::Bytes(bytes) => bytes,
		Body::Stream(_) => {
			return Err(bevyhow!(
				"cannot encode a streaming request body; collect it into bytes first"
			));
		}
	};

	let path = request.path_string();
	let query = request.query_string();
	let target = if query.is_empty() {
		path
	} else {
		format!("{path}?{query}")
	};

	let mut head = String::new();
	write!(
		head,
		"{} {} HTTP/1.1\r\n",
		method_token(request.method()),
		target
	)
	.ok();
	write!(head, "Host: {}\r\n", request.authority()).ok();
	for (key, values) in request.headers().iter_all() {
		if is_managed_header(key) {
			continue;
		}
		for value in values {
			write!(head, "{key}: {value}\r\n").ok();
		}
	}
	write!(head, "Content-Length: {}\r\n", body.len()).ok();
	head.push_str("Connection: close\r\n\r\n");

	let mut bytes = head.into_bytes();
	bytes.extend_from_slice(body);
	Ok(bytes)
}

/// Parse raw HTTP/1.1 response bytes into a beet [`Response`].
///
/// The inverse of [`serialize_http_response`]: reads the status line, the
/// headers, and the remaining bytes as the body. Reuses [`find_header_end`] to
/// locate the header/body boundary, mirroring [`parse_http_request`].
pub fn parse_response(raw: &[u8]) -> Result<Response> {
	let (header_bytes, body_bytes) = match find_header_end(raw) {
		Some(end) => (&raw[..end], &raw[end..]),
		None => (raw, &[][..]),
	};
	let header_str = String::from_utf8_lossy(header_bytes);
	let mut lines = header_str.lines();

	// Parse the status line: HTTP/VERSION CODE [REASON]
	let status_line = lines.next().ok_or_else(|| bevyhow!("empty response"))?;
	let status = status_line
		.split_whitespace()
		.nth(1)
		.and_then(|code| code.parse::<u16>().ok())
		.ok_or_else(|| bevyhow!("missing or invalid HTTP status code"))?;

	let mut parts = ResponseParts::new(StatusCode::new(status));
	for line in lines {
		if line.is_empty() {
			break;
		}
		if let Some((key, value)) = line.split_once(':') {
			parts.headers.set_raw(key.trim(), value.trim().to_string());
		}
	}

	Ok(Response::new(parts, body_bytes.to_vec().into()))
}

/// Serialize a beet [`Response`] into raw HTTP/1.1 bytes.
///
/// Drains the response body (collecting any stream into a single buffer) and
/// writes the status line, headers, a computed `content-length`, and a
/// `connection: close`. Streaming/chunked transfer is left to backends that
/// can drive it incrementally over their own socket type.
pub async fn serialize_http_response(response: Response) -> Result<Vec<u8>> {
	let (parts, mut body) = response.into_parts();

	// Collect the body
	let mut body_bytes = Vec::new();
	while let Some(chunk) = body.next().await? {
		body_bytes.extend_from_slice(&chunk);
	}

	let status_code = parts.status();

	let mut head = String::new();

	// Status line
	write!(
		head,
		"HTTP/1.1 {} {}\r\n",
		status_code.as_u16(),
		status_code.message()
	)
	.ok();

	// Headers from response parts
	for (key, values) in parts.headers().iter_all() {
		for value in values {
			write!(head, "{}: {}\r\n", key, value).ok();
		}
	}

	// Content-Length header
	write!(head, "content-length: {}\r\n", body_bytes.len()).ok();

	// Ensure Connection: close for simplicity
	head.push_str("connection: close\r\n");

	// End of headers
	head.push_str("\r\n");

	let mut output = head.into_bytes();
	output.extend_from_slice(&body_bytes);

	Ok(output)
}

/// Whether request headers ask to upgrade the connection to a WebSocket.
///
/// Pure and transport-agnostic: `Connection` must list `upgrade` (it may carry
/// other tokens, eg `keep-alive, Upgrade`) and `Upgrade` must name `websocket`,
/// both matched case-insensitively per [RFC 6455 §4.2.1].
pub fn is_websocket_upgrade(headers: &HeaderMap) -> bool {
	let connection_upgrades = headers
		.first_raw("connection")
		.map(|val| {
			val.split(',')
				.any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
		})
		.unwrap_or(false);
	let upgrade_websocket = headers
		.first_raw("upgrade")
		.map(|val| {
			val.split(',')
				.any(|token| token.trim().eq_ignore_ascii_case("websocket"))
		})
		.unwrap_or(false);
	connection_upgrades && upgrade_websocket
}

/// The client's `Sec-WebSocket-Key` header value, required to compute the
/// handshake response (see [`sec_websocket_accept`]).
pub fn sec_websocket_key(headers: &HeaderMap) -> Option<&str> {
	headers.first_raw("sec-websocket-key")
}

/// Compute the `Sec-WebSocket-Accept` value for a client's `Sec-WebSocket-Key`,
/// `base64(SHA1(key + GUID))` per [RFC 6455 §4.2.2].
///
/// Reuses the workspace `sha1` (the RustCrypto sibling of `sha2`, used for the
/// `aws_sdk` signing) and `base64` crates, so no new crypto dependency is added.
///
/// Rides the no_std `sockets` feature (not `tungstenite`): the bare-metal client
/// (`ws_ext`) computes it to validate the server's `101` handshake response.
#[cfg(feature = "sockets")]
pub fn sec_websocket_accept(key: &str) -> String {
	/// The GUID concatenated with `Sec-WebSocket-Key` before hashing, per
	/// [RFC 6455 §1.3](https://datatracker.ietf.org/doc/html/rfc6455#section-1.3).
	const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

	use base64::Engine as _;
	use sha1::Digest;
	let mut hasher = sha1::Sha1::new();
	hasher.update(key.as_bytes());
	hasher.update(WEBSOCKET_GUID.as_bytes());
	base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

/// A response that signals the backend to upgrade the connection to a WebSocket
/// rather than write a normal body.
///
/// A route returns `WebSocketUpgrade::new(request)` (or `from_request`); it
/// lowers to a `101 Switching Protocols` [`Response`] carrying the computed
/// `Sec-WebSocket-Accept` and the required `Upgrade`/`Connection` headers. The
/// `101` status is the backend's signal to keep the raw stream and hand it to
/// the socket layer (see `mini_http_server`/`hyper_server`), instead of closing
/// the connection after the body.
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
#[derive(Debug, Clone)]
pub struct WebSocketUpgrade {
	/// The computed `Sec-WebSocket-Accept` value, or `None` when the request was
	/// not a valid upgrade (a missing/invalid `Sec-WebSocket-Key`).
	accept: Option<String>,
}

#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
impl WebSocketUpgrade {
	/// Build the upgrade from request headers, computing the accept key.
	pub fn from_request(request: &Request) -> Self {
		Self::new(request.headers())
	}

	/// Build the upgrade from request headers, computing the accept key.
	pub fn new(headers: &HeaderMap) -> Self {
		let accept = is_websocket_upgrade(headers)
			.then(|| sec_websocket_key(headers))
			.flatten()
			.map(sec_websocket_accept);
		Self { accept }
	}

	/// Whether the request was a valid upgrade (had the headers and a key).
	pub fn is_valid(&self) -> bool { self.accept.is_some() }

	/// Lower into the `101 Switching Protocols` handshake [`Response`], or a
	/// `400 Bad Request` if the request was not a valid upgrade.
	pub fn into_response(self) -> Response {
		let Some(accept) = self.accept else {
			return Response::from_status(StatusCode::BAD_REQUEST);
		};
		let mut parts = ResponseParts::new(StatusCode::SWITCHING_PROTOCOLS);
		parts.headers.set_raw("upgrade", "websocket");
		parts.headers.set_raw("connection", "Upgrade");
		parts.headers.set_raw("sec-websocket-accept", accept);
		Response::new(parts, Body::default())
	}
}

#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
impl From<WebSocketUpgrade> for Response {
	fn from(upgrade: WebSocketUpgrade) -> Self { upgrade.into_response() }
}

/// Whether a [`Response`] is a WebSocket upgrade handshake (a `101` with the
/// `upgrade: websocket` header), ie a backend should keep the raw stream and
/// hand it to the socket layer instead of closing after the body.
pub fn is_websocket_response(response: &Response) -> bool {
	response.status() == StatusCode::SWITCHING_PROTOCOLS
		&& response
			.headers()
			.first_raw("upgrade")
			.map(|val| val.eq_ignore_ascii_case("websocket"))
			.unwrap_or(false)
}

/// Check if HTTP request parts indicate a body is present based on headers.
#[cfg(feature = "http")]
pub fn has_body(parts: &http::request::Parts) -> bool {
	has_body_by_content_length(&parts.headers)
		|| has_body_by_transfer_encoding(&parts.headers)
}

/// Check if headers indicate a body by content-length > 0.
#[cfg(feature = "http")]
pub fn has_body_by_content_length(headers: &http::HeaderMap) -> bool {
	headers
		.get("content-length")
		.and_then(|val| val.to_str().ok())
		.and_then(|str| str.parse::<usize>().ok())
		.map(|len| len > 0)
		.unwrap_or(false)
}

/// Check if headers indicate a body by chunked transfer encoding.
#[cfg(feature = "http")]
pub fn has_body_by_transfer_encoding(headers: &http::HeaderMap) -> bool {
	headers
		.get("transfer-encoding")
		.and_then(|val| val.to_str().ok())
		.map(|str| str.contains("chunked"))
		.unwrap_or(false)
}

/// Convert http version to string representation.
#[cfg(feature = "http")]
pub fn version_to_string(version: http::Version) -> String {
	match version {
		http::Version::HTTP_09 => "0.9".to_string(),
		http::Version::HTTP_10 => "1.0".to_string(),
		http::Version::HTTP_11 => "1.1".to_string(),
		http::Version::HTTP_2 => "2".to_string(),
		http::Version::HTTP_3 => "3".to_string(),
		_ => "1.1".to_string(),
	}
}

/// Parse a version string into an http::Version.
#[cfg(feature = "http")]
pub fn parse_version(version: &str) -> http::Version {
	match version {
		"0.9" => http::Version::HTTP_09,
		"1.0" => http::Version::HTTP_10,
		"1.1" => http::Version::HTTP_11,
		"2" | "2.0" => http::Version::HTTP_2,
		"3" | "3.0" => http::Version::HTTP_3,
		_ => http::Version::HTTP_11,
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::header;

	// -- parse_http_request --

	#[beet_core::test]
	fn parse_get_request() {
		let raw = b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
		let request = parse_http_request(raw).unwrap();
		request.method().xpect_eq(HttpMethod::Get);
		request.path_string().as_str().xpect_eq("/hello");
	}

	#[beet_core::test]
	fn parse_post_with_body() {
		let raw =
			b"POST /api HTTP/1.1\r\nContent-Type: text/plain\r\n\r\nhello body";
		let request = parse_http_request(raw).unwrap();
		request.method().xpect_eq(HttpMethod::Post);
		request.path_string().as_str().xpect_eq("/api");
	}

	#[beet_core::test]
	fn parse_with_accept_header() {
		let raw =
			b"GET / HTTP/1.1\r\nAccept: text/html\r\nHost: localhost\r\n\r\n";
		let request = parse_http_request(raw).unwrap();
		let accepted =
			request.headers.get::<header::Accept>().unwrap().unwrap();
		accepted.len().xpect_eq(1);
		accepted[0].xpect_eq(MediaType::Html);
	}

	#[beet_core::test]
	fn parse_query_string() {
		let raw = b"GET /search?q=hello HTTP/1.1\r\n\r\n";
		let request = parse_http_request(raw).unwrap();
		request.path_string().as_str().xpect_eq("/search");
	}

	#[beet_core::test]
	fn parse_empty_returns_error() { parse_http_request(b"").xpect_err(); }

	// -- encode_request --

	#[beet_core::test]
	fn encode_get_roundtrips_through_parse() {
		let request = Request::get("http://localhost/hello?q=world");
		let raw = encode_request(&request).unwrap();
		let raw_str = String::from_utf8(raw.clone()).unwrap();
		raw_str
			.as_str()
			.xpect_contains("GET /hello?q=world HTTP/1.1");
		raw_str.as_str().xpect_contains("Host: localhost");
		raw_str.as_str().xpect_contains("Content-Length: 0");

		// the encoder's output parses back into an equivalent request
		let parsed = parse_http_request(&raw).unwrap();
		parsed.method().xpect_eq(HttpMethod::Get);
		parsed.path_string().as_str().xpect_eq("/hello");
	}

	#[beet_core::test]
	fn encode_post_with_body() {
		let mut request = Request::post("http://localhost/api");
		request.set_body("hello body");
		let raw = encode_request(&request).unwrap();
		let raw_str = String::from_utf8(raw.clone()).unwrap();
		raw_str.as_str().xpect_contains("POST /api HTTP/1.1");
		raw_str.as_str().xpect_contains("Content-Length: 10");
		raw_str.as_str().xpect_contains("hello body");

		let parsed = parse_http_request(&raw).unwrap();
		parsed.method().xpect_eq(HttpMethod::Post);
	}

	#[cfg(feature = "std")]
	#[beet_core::test]
	fn encode_rejects_streaming_body() {
		let stream =
			futures::stream::once(async { Ok(bytes::Bytes::from("x")) });
		let mut request = Request::post("http://localhost/api");
		request.body = Body::stream(stream);
		encode_request(&request).xpect_err();
	}

	// -- parse_response --

	#[beet_core::test]
	fn parse_ok_response() {
		let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nhello";
		let response = parse_response(raw).unwrap();
		response.status().as_u16().xpect_eq(200);
		let body = response.body.try_into_bytes().unwrap();
		core::str::from_utf8(&body).unwrap().xpect_eq("hello");
	}

	#[beet_core::test]
	fn parse_response_no_body() {
		let raw = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
		let response = parse_response(raw).unwrap();
		response.status().as_u16().xpect_eq(404);
	}

	#[beet_core::test]
	fn parse_response_empty_returns_error() { parse_response(b"").xpect_err(); }

	// -- serialize_http_response --

	#[beet_core::test]
	async fn serialize_ok_response() {
		let response = Response::ok_body("hello", MediaType::Text);
		let raw = serialize_http_response(response).await.unwrap();
		let raw_str = String::from_utf8(raw).unwrap();
		raw_str.as_str().xpect_contains("HTTP/1.1 200 OK");
		raw_str.as_str().xpect_contains("content-length: 5");
		raw_str.as_str().xpect_contains("hello");
	}

	#[beet_core::test]
	async fn serialize_empty_response() {
		let response = Response::ok();
		let raw = serialize_http_response(response).await.unwrap();
		let raw_str = String::from_utf8(raw).unwrap();
		raw_str.as_str().xpect_contains("HTTP/1.1 200 OK");
		raw_str.as_str().xpect_contains("content-length: 0");
	}

	// -- websocket upgrade --

	/// Headers carrying a valid upgrade request, with the RFC 6455 example key.
	fn upgrade_headers() -> HeaderMap {
		let mut headers = HeaderMap::new();
		headers.set_raw("upgrade", "websocket");
		headers.set_raw("connection", "Upgrade");
		headers.set_raw("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==");
		headers
	}

	#[beet_core::test]
	fn detects_upgrade_headers() {
		is_websocket_upgrade(&upgrade_headers()).xpect_true();
	}

	#[beet_core::test]
	fn detects_upgrade_case_insensitively_with_extra_tokens() {
		let mut headers = HeaderMap::new();
		// real browsers send a multi-token Connection and mixed casing
		headers.set_raw("upgrade", "WebSocket");
		headers.set_raw("connection", "keep-alive, Upgrade");
		is_websocket_upgrade(&headers).xpect_true();
	}

	#[beet_core::test]
	fn rejects_non_upgrade_headers() {
		let mut headers = HeaderMap::new();
		headers.set_raw("connection", "keep-alive");
		is_websocket_upgrade(&headers).xpect_false();
		is_websocket_upgrade(&HeaderMap::new()).xpect_false();
	}

	// RFC 6455 §1.3 worked example: key `dGhlIHNhbXBsZSBub25jZQ==` hashes to the
	// accept value `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`.
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn computes_rfc_accept_key() {
		sec_websocket_accept("dGhlIHNhbXBsZSBub25jZQ==")
			.xpect_eq("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
	}

	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn upgrade_lowers_to_101_handshake() {
		let upgrade = WebSocketUpgrade::new(&upgrade_headers());
		upgrade.is_valid().xpect_true();
		let response = upgrade.into_response();
		response.status().xpect_eq(StatusCode::SWITCHING_PROTOCOLS);
		response
			.headers()
			.first_raw("sec-websocket-accept")
			.unwrap()
			.xpect_eq("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
		is_websocket_response(&response).xpect_true();
	}

	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	#[beet_core::test]
	fn invalid_upgrade_lowers_to_400() {
		// no `Sec-WebSocket-Key`: not a valid handshake
		let mut headers = HeaderMap::new();
		headers.set_raw("upgrade", "websocket");
		headers.set_raw("connection", "Upgrade");
		let upgrade = WebSocketUpgrade::new(&headers);
		upgrade.is_valid().xpect_false();
		upgrade
			.into_response()
			.status()
			.xpect_eq(StatusCode::BAD_REQUEST);
	}

	// -- http-crate helpers --

	#[cfg(feature = "http")]
	#[beet_core::test]
	fn has_body_with_content_length() {
		let parts = http::Request::builder()
			.method(http::Method::POST)
			.uri("/test")
			.header("content-length", "5")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		has_body(&parts).xpect_true();
	}

	#[cfg(feature = "http")]
	#[beet_core::test]
	fn has_body_without_headers() {
		let parts = http::Request::builder()
			.method(http::Method::GET)
			.uri("/test")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		has_body(&parts).xpect_false();
	}

	#[cfg(feature = "http")]
	#[beet_core::test]
	fn has_body_with_chunked_encoding() {
		let parts = http::Request::builder()
			.method(http::Method::POST)
			.uri("/test")
			.header("transfer-encoding", "chunked")
			.body(())
			.unwrap()
			.into_parts()
			.0;

		has_body(&parts).xpect_true();
	}

	#[cfg(feature = "http")]
	#[beet_core::test]
	fn version_conversions() {
		version_to_string(http::Version::HTTP_11).xpect_eq("1.1");
		version_to_string(http::Version::HTTP_2).xpect_eq("2");
		parse_version("1.1").xpect_eq(http::Version::HTTP_11);
		parse_version("2").xpect_eq(http::Version::HTTP_2);
	}
}
