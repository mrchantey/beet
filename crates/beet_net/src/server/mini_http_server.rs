//! Minimal HTTP/1.1 server using `async-io` TCP.
//!
//! This is a lightweight alternative to the hyper-based server that
//! requires no additional dependencies beyond `async-io` and
//! `futures-lite`. It parses raw HTTP/1.1 requests, dispatches them
//! through the entity's tool pipeline, and writes raw HTTP responses
//! back to the connection.
//!
//! Selected automatically when the `server` feature is enabled but
//! neither `hyper` nor `lambda` features are active.
use crate::prelude::*;
use beet_core::prelude::*;
use std::io::Write;
use std::net::SocketAddr;

/// Start a mini HTTP server on the entity's [`HttpServer`] address.
///
/// This system mirrors the signature of `start_hyper_server` and
/// `start_lambda_server` so the `HttpServer` component can swap
/// backends via feature flags.
pub(super) fn start_mini_http_server(
	In(entity): In<Entity>,
	query: Query<&HttpServer>,
	mut async_commands: AsyncCommands,
) -> Result {
	let server = query.get(entity)?;
	let addr: SocketAddr = (server.host, server.port).into();

	async_commands.run(async move |world| -> Result {
		let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
			.map_err(|err| {
				bevyhow!("Failed to bind mini HTTP server to {addr}: {err}")
			})?;

		cross_log!("Mini HTTP server listening on http://{addr}");

		loop {
			let accept_result = listener.accept().await;
			let (stream, peer_addr) = match accept_result {
				Ok(pair) => pair,
				Err(err) => {
					cross_log_error!("Failed to accept connection: {err}");
					continue;
				}
			};

			let _entity_fut = world.run_async(async move |world| {
				if let Err(err) =
					handle_connection(world.entity(entity), stream, peer_addr)
						.await
				{
					cross_log_error!(
						"Error handling connection from {peer_addr}: {err}"
					);
				}
			});
		}
	});
	Ok(())
}


/// Handle a single HTTP connection: read the request, dispatch it,
/// and write the response.
async fn handle_connection(
	entity: AsyncEntity,
	mut stream: async_io::Async<std::net::TcpStream>,
	_peer_addr: SocketAddr,
) -> Result {
	use futures_lite::AsyncReadExt;
	use futures_lite::AsyncWriteExt;

	// Read the raw HTTP request
	let mut buf = vec![0u8; 8192];
	let bytes_read = stream.read(&mut buf).await?;
	if bytes_read == 0 {
		return Ok(());
	}
	buf.truncate(bytes_read);

	// Parse the raw HTTP request into our Request type
	let request = parse_http_request(&buf)?;

	// Dispatch through the entity's exchange
	let response: Response = entity.exchange(request).await;

	// Serialize the response and write it back
	let raw_response = serialize_http_response(response).await?;
	stream.write_all(&raw_response).await?;
	stream.flush().await?;

	Ok(())
}

/// Parse a raw HTTP request into a [`Request`].
pub(crate) fn parse_http_request(raw: &[u8]) -> Result<Request> {
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

/// Serialize a [`Response`] into raw HTTP/1.1 bytes.
pub(crate) async fn serialize_http_response(
	response: Response,
) -> Result<Vec<u8>> {
	let (parts, mut body) = response.into_parts();

	// Collect the body
	let mut body_bytes = Vec::new();
	while let Some(chunk) = body.next().await? {
		body_bytes.extend_from_slice(&chunk);
	}

	let status_code = parts.status();

	let mut output = Vec::new();

	// Status line
	write!(
		output,
		"HTTP/1.1 {} {}\r\n",
		status_code.as_u16(),
		status_code.message()
	)?;

	// Headers from response parts
	for (key, values) in parts.headers().iter_all() {
		for value in values {
			write!(output, "{}: {}\r\n", key, value)?;
		}
	}

	// Content-Length header
	write!(output, "content-length: {}\r\n", body_bytes.len())?;

	// Ensure Connection: close for simplicity
	write!(output, "connection: close\r\n")?;

	// End of headers
	write!(output, "\r\n")?;

	// Body
	output.extend_from_slice(&body_bytes);

	Ok(output)
}


#[cfg(test)]
mod test {
	use super::*;
	use beet_core::header;

	// -- parse_http_request --

	#[test]
	fn parse_get_request() {
		let raw = b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
		let request = parse_http_request(raw).unwrap();
		request.method().xpect_eq(HttpMethod::Get);
		request.path_string().as_str().xpect_eq("/hello");
	}

	#[test]
	fn parse_post_with_body() {
		let raw =
			b"POST /api HTTP/1.1\r\nContent-Type: text/plain\r\n\r\nhello body";
		let request = parse_http_request(raw).unwrap();
		request.method().xpect_eq(HttpMethod::Post);
		request.path_string().as_str().xpect_eq("/api");
	}

	#[test]
	fn parse_with_accept_header() {
		let raw =
			b"GET / HTTP/1.1\r\nAccept: text/html\r\nHost: localhost\r\n\r\n";
		let request = parse_http_request(raw).unwrap();
		let accepted =
			request.headers.get::<header::Accept>().unwrap().unwrap();
		accepted.len().xpect_eq(1);
		accepted[0].xpect_eq(MediaType::Html);
	}

	#[test]
	fn parse_query_string() {
		let raw = b"GET /search?q=hello HTTP/1.1\r\n\r\n";
		let request = parse_http_request(raw).unwrap();
		request.path_string().as_str().xpect_eq("/search");
	}

	#[test]
	fn parse_empty_returns_error() { parse_http_request(b"").xpect_err(); }

	// -- serialize_http_response --

	#[test]
	fn serialize_ok_response() {
		let response = Response::ok_body("hello", MediaType::Text);
		let raw =
			async_ext::block_on(serialize_http_response(response)).unwrap();
		let raw_str = String::from_utf8(raw).unwrap();
		raw_str.as_str().xpect_contains("HTTP/1.1 200 OK");
		raw_str.as_str().xpect_contains("content-length: 5");
		raw_str.as_str().xpect_contains("hello");
	}

	#[test]
	fn serialize_empty_response() {
		let response = Response::ok();
		let raw =
			async_ext::block_on(serialize_http_response(response)).unwrap();
		let raw_str = String::from_utf8(raw).unwrap();
		raw_str.as_str().xpect_contains("HTTP/1.1 200 OK");
		raw_str.as_str().xpect_contains("content-length: 0");
	}
}
