//! Minimal HTTP/1.1 server using `async-io` TCP.
//!
//! This is a lightweight alternative to the hyper-based server that
//! requires no additional dependencies beyond `async-io` and
//! `futures-lite`. It parses raw HTTP/1.1 requests, dispatches them
//! through the entity's action pipeline, and writes raw HTTP responses
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
/// This async function mirrors the signature of `start_hyper_server` and
/// `start_lambda_server` so the `HttpServer` component can swap
/// backends via feature flags.
pub async fn start_mini_http_server(entity: AsyncEntity) -> Result {
	let addr: SocketAddr = entity
		.get::<HttpServer, SocketAddr>(|server| {
			(server.host, server.port.unwrap_or(0)).into()
		})
		.await?;

	let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
		.map_err(|err| {
			bevyhow!("Failed to bind mini HTTP server to {addr}: {err}")
		})?;

	start_mini_http_server_with_tcp(entity, listener).await
}

/// Start a mini HTTP server using a pre-bound TCP listener.
///
/// This variant accepts an already-bound listener, which eliminates
/// port race conditions in tests. See [`start_mini_http_server`] for
/// the convenience wrapper that binds its own listener.
pub async fn start_mini_http_server_with_tcp(
	entity: AsyncEntity,
	listener: async_io::Async<std::net::TcpListener>,
) -> Result {
	let addr = listener
		.get_ref()
		.local_addr()
		.map_err(|err| bevyhow!("Failed to get local address: {err}"))?;
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

		entity
			.run_async(async move |entity| {
				if let Err(err) =
					handle_connection(entity, stream, peer_addr).await
				{
					cross_log_error!(
						"Error handling connection from {peer_addr}: {err}"
					);
				}
			})
			.await
			.ok();
	}
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

	// Read the raw HTTP request headers (and possibly partial body)
	let mut buf = vec![0u8; 8192];
	let bytes_read = stream.read(&mut buf).await?;
	if bytes_read == 0 {
		return Ok(());
	}
	buf.truncate(bytes_read);

	// Check if we need to read more bytes based on Content-Length
	let header_end = http_ext::find_header_end(&buf);
	if let Some(header_end_pos) = header_end {
		let content_length =
			http_ext::parse_content_length(&buf[..header_end_pos]);
		if content_length > 0 {
			let body_start = header_end_pos;
			let body_received = buf.len() - body_start;
			let remaining = content_length.saturating_sub(body_received);
			if remaining > 0 {
				buf.resize(body_start + content_length, 0);
				let mut total_read = body_received;
				while total_read < content_length {
					let read_count = stream
						.read(&mut buf[body_start + total_read..])
						.await?;
					if read_count == 0 {
						break;
					}
					total_read += read_count;
				}
				buf.truncate(body_start + total_read);
			}
		}
	}

	// Parse the raw HTTP request into our Request type
	let request = http_ext::parse_http_request(&buf)?;

	// Dispatch through the entity's exchange
	let response: Response = entity.exchange(request).await;
	let (parts, body) = response.into_parts();

	match body {
		Body::Bytes(bytes) => {
			// Use standard serialization for non-streaming responses
			let response = Response {
				parts,
				body: Body::Bytes(bytes),
			};
			let raw_response =
				http_ext::serialize_http_response(response).await?;
			stream.write_all(&raw_response).await?;
			stream.flush().await?;
		}
		Body::Stream(body_stream) => {
			// Write status line and headers with chunked transfer encoding
			let status_code = parts.status();
			let mut header_buf = Vec::new();
			write!(
				header_buf,
				"HTTP/1.1 {} {}\r\n",
				status_code.as_u16(),
				status_code.message()
			)?;
			for (key, values) in parts.headers().iter_all() {
				for value in values {
					write!(header_buf, "{}: {}\r\n", key, value)?;
				}
			}
			write!(header_buf, "transfer-encoding: chunked\r\n")?;
			write!(header_buf, "connection: close\r\n")?;
			write!(header_buf, "\r\n")?;
			stream.write_all(&header_buf).await?;

			// Write each chunk in HTTP chunked transfer encoding
			let mut body = Body::Stream(body_stream);
			while let Some(chunk) = body.next().await? {
				let chunk_header = format!("{:x}\r\n", chunk.len());
				stream.write_all(chunk_header.as_bytes()).await?;
				stream.write_all(&chunk).await?;
				stream.write_all(b"\r\n").await?;
				stream.flush().await?;
			}
			// Terminating zero-length chunk
			stream.write_all(b"0\r\n\r\n").await?;
			stream.flush().await?;
		}
	}

	Ok(())
}


#[cfg(test)]
mod test {
	use super::*;

	// -- integration test via shared suite --
	// (pure parse/serialise unit tests live with the shared helpers in
	// `crate::types::http_ext`.)

	#[cfg(feature = "ureq")]
	#[beet_core::test]
	async fn roundtrip() {
		super::super::http_server::test::test_server(
			start_mini_http_server_with_tcp,
		)
		.await;
	}
}
