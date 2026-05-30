//! Pure HTTP/1.1 wire helpers shared across server backends.
//!
//! These operate only on byte buffers and beet's [`Request`]/[`Response`] wire
//! types, so they are `no_std` and shared by the std `mini_http_server` and any
//! downstream embedded backend (eg an esp/embassy WiFi server) installed via
//! [`set_http_server`]. Only the *pure* parse/serialise half is shareable; the
//! connection + streaming half differs per transport (std `async-io` vs an
//! embassy `TcpSocket`) and stays in each backend.

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
}
