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
pub async fn start_mini_http_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	let addr: SocketAddr = entity
		.get::<HttpServer, SocketAddr>(|server| server.socket_addr())
		.await?;

	let listener = async_io::Async::<std::net::TcpListener>::bind(addr)
		.map_err(|err| {
			bevyhow!("Failed to bind mini HTTP server to {addr}: {err}")
		})?;

	start_mini_http_server_with_tcp(entity, listener, shutdown).await
}

/// Start a mini HTTP server using a pre-bound TCP listener.
///
/// This variant accepts an already-bound listener, which eliminates
/// port race conditions in tests. See [`start_mini_http_server`] for
/// the convenience wrapper that binds its own listener.
pub async fn start_mini_http_server_with_tcp(
	entity: AsyncEntity,
	listener: async_io::Async<std::net::TcpListener>,
	shutdown: OnceValueRx<()>,
) -> Result {
	let addr = listener
		.get_ref()
		.local_addr()
		.map_err(|err| bevyhow!("Failed to get local address: {err}"))?;
	info!("Mini HTTP server listening on http://{addr}");

	// race the accept loop against the shutdown signal: when teardown signals,
	// the loop future is dropped, releasing the listener so the port closes. The
	// per-connection tasks are spawned, so this is a minimal drain — in-flight
	// requests finish on their own (or are cut by process exit when nothing else
	// holds the process up).
	beet_core::exports::futures_lite::future::or(
		accept_loop(entity, listener),
		async move {
			shutdown.wait().await;
			Result::Ok(())
		},
	)
	.await
}

/// The accept loop: dispatch each connection on its own spawned task. Diverges
/// (only [`start_mini_http_server_with_tcp`]'s shutdown race ends it).
async fn accept_loop(
	entity: AsyncEntity,
	listener: async_io::Async<std::net::TcpListener>,
) -> Result {
	loop {
		let accept_result = listener.accept().await;
		let (stream, peer_addr) = match accept_result {
			Ok(pair) => pair,
			Err(err) => {
				error!("Failed to accept connection: {err}");
				continue;
			}
		};

		entity
			.run_async(async move |entity| {
				if let Err(err) =
					handle_connection(entity, stream, peer_addr).await
				{
					error!("Error handling connection from {peer_addr}: {err}");
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

	// Dispatch through the host's routing
	let response: Response = entity.exchange(request).await;

	// A `101 Switching Protocols` (a route returning `WebSocketUpgrade`) means we
	// write the handshake then keep the raw stream as a `Socket`, instead of
	// closing after the body.
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	if http_ext::is_websocket_response(&response) {
		return upgrade_connection(entity, stream, response).await;
	}

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

/// Complete a WebSocket upgrade on a raw connection: write the `101` handshake
/// bytes, wrap the stream as a [`Socket`] (`Role::Server`, no re-handshake), and
/// trigger [`OnWebSocketUpgrade`] so the socket layer (eg `client_io`) can adopt
/// it. The `client_io` broadcast/registry layer is unchanged: it sees a normal
/// `Socket` entity.
///
/// The whole hand-off runs `_local` on the world-owning thread, where the
/// `Socket`'s thread-bound `SendWrapper` reader is created and polled, mirroring
/// the side-port [`start_tungstenite_server`](crate::sockets) accept loop.
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
async fn upgrade_connection(
	entity: AsyncEntity,
	stream: async_io::Async<std::net::TcpStream>,
	response: Response,
) -> Result {
	// write the handshake by hand: a `101` keeps the connection open, so it must
	// not get the `content-length`/`connection: close` `serialize_http_response`
	// appends for a normal body.
	let parts = response.into_parts().0;
	let mut handshake = Vec::new();
	write!(
		handshake,
		"HTTP/1.1 {} {}\r\n",
		parts.status().as_u16(),
		parts.status().message()
	)?;
	for (key, values) in parts.headers().iter_all() {
		for value in values {
			write!(handshake, "{key}: {value}\r\n")?;
		}
	}
	write!(handshake, "\r\n")?;
	entity
		.run_async_local(async move |entity| -> Result {
			use futures_lite::AsyncWriteExt;
			let mut stream = stream;
			stream.write_all(&handshake).await?;
			stream.flush().await?;
			// wrap the now-upgraded stream, spawn it as a `Socket`, and announce it
			let socket = crate::sockets::socket_from_upgraded(stream).await;
			entity
				.world()
				.with(move |world: &mut World| {
					let socket = world.spawn(socket).id();
					world
						.trigger(crate::sockets::OnWebSocketUpgrade { socket });
				})
				.await;
			Ok(())
		})
		.await?;
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

	/// The same-port upgrade: a route returning [`WebSocketUpgrade`] hands the
	/// raw stream to the socket layer as a [`Socket`] entity, and the channel
	/// echoes over it. This is the seam `client_io` rides off the side port.
	#[cfg(feature = "tungstenite")]
	#[beet_core::test]
	async fn upgrades_to_socket() {
		use crate::sockets::*;

		let server = HttpServer::new_test(start_mini_http_server_with_tcp);
		let port = server.0.port.unwrap();
		// records each landed socket entity so the test can assert the upgrade
		let landed = Store::<Vec<Entity>>::default();
		let captor = landed.clone();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			// a route that upgrades any request to a websocket
			app.world_mut().spawn((
				server,
				DispatchExchange(exchange_handler(|cx| {
					WebSocketUpgrade::from_request(&cx).into()
				})),
			));
			// record landed sockets
			app.world_mut()
				.add_observer(move |ev: On<OnWebSocketUpgrade>| {
					captor.push(ev.event().socket);
				});
			// a global recv observer echoes text back; global (not per-socket)
			// so it is always installed before the socket reader fires, avoiding
			// a deferred-registration race
			app.world_mut().add_observer(
				|ev: On<MessageRecv>, mut commands: Commands| {
					if let Message::Text(text) = ev.event().inner() {
						commands.entity(ev.original_target()).trigger_target(
							MessageSend(Message::text(text.clone())),
						);
					}
				},
			);
			app.run();
		});
		time_ext::sleep_millis(200).await;

		// a real client connects over the main HTTP port and upgrades
		let mut client = Socket::connect(format!("ws://127.0.0.1:{port}"))
			.await
			.unwrap();
		client
			.send(Message::text("over-the-upgrade"))
			.await
			.unwrap();

		// the server echoes the message back over the upgraded channel
		let mut echoed = None;
		for _ in 0..40 {
			if let Some(Ok(Message::Text(text))) = client.next().await {
				echoed = Some(text);
				break;
			}
		}
		echoed.xpect_eq(Some("over-the-upgrade".to_string()));
		// exactly one socket entity landed for the one connection
		landed.get().len().xpect_eq(1usize);
		client.close(None).await.ok();
	}
}
