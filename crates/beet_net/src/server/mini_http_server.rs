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
use bytes::Bytes;
use std::io::Write;
use std::net::SocketAddr;

impl HttpServer {
	/// Start a mini HTTP server on the entity's [`HttpServer`] address.
	///
	/// This async function mirrors the signature of `HttpServer::start_hyper` and
	/// `HttpServer::start_lambda` so the `HttpServer` component can swap
	/// backends via feature flags.
	pub async fn start_mini(
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

		HttpServer::start_mini_with_tcp(entity, listener, shutdown).await
	}
}
impl HttpServer {
	/// Start a mini HTTP server using a pre-bound TCP listener.
	///
	/// This variant accepts an already-bound listener, which eliminates
	/// port race conditions in tests. See [`HttpServer::start_mini`] for
	/// the convenience wrapper that binds its own listener.
	pub async fn start_mini_with_tcp(
		entity: AsyncEntity,
		listener: async_io::Async<std::net::TcpListener>,
		shutdown: OnceValueRx<()>,
	) -> Result {
		let addr = listener
			.get_ref()
			.local_addr()
			.map_err(|err| bevyhow!("Failed to get local address: {err}"))?;
		// build the TLS acceptor (if any) before logging so the printed scheme is real
		let tls = MaybeTls::resolve(&entity).await?;
		info!(
			"Mini HTTP server listening on {}://{addr}",
			tls.http_scheme()
		);
		// the bound address as data on the entity, and the process loopback port
		// when canonical
		Listening::register(&entity, addr).await?;

		// the declared caps and their shared permits, resolved once: every
		// connection below is served under this one guard
		let guard = ServerGuard::resolve(&entity).await?;

		// race the accept loop against the shutdown signal: when teardown signals,
		// the loop future is dropped, releasing the listener so the port closes. The
		// per-connection tasks are spawned, so this is a minimal drain — in-flight
		// requests finish on their own (or are cut by process exit when nothing else
		// holds the process up).
		let served = beet_core::exports::futures_lite::future::or(
			accept_loop(entity, listener, tls, guard),
			async move {
				shutdown.wait().await;
				Result::Ok(())
			},
		)
		.await;
		debug!("Mini HTTP server on {addr} shut down");
		served
	}
}
/// The accept loop: dispatch each connection on its own task, spawned on the
/// server entity. Diverges (only [`HttpServer::start_mini_with_tcp`]'s
/// shutdown race ends it).
///
/// A connection task is entity-scoped, so it is cancelled when the server
/// entity despawns and its stream closes with it: a browser's idle
/// preconnect, reused for the request after a live-reload rebuild, sees EOF
/// and retries on the rebuilt listener rather than being answered by the
/// torn-down entity with a 500.
async fn accept_loop(
	entity: AsyncEntity,
	listener: async_io::Async<std::net::TcpListener>,
	tls: MaybeTls,
	guard: ServerGuard,
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

		let tls = tls.clone();
		let guard = guard.clone();
		entity
			.run_async(async move |entity| {
				if let Err(err) =
					serve_sniffed(entity, stream, peer_addr, tls, guard).await
				{
					error!("Error handling connection from {peer_addr}: {err}");
				}
			})
			.await
			.ok();
	}
}

/// Classify the connection's first bytes and dispatch: TLS is accepted onto
/// the regular handler, plaintext is served for loopback peers (localhost is
/// already a secure context, and the reload watcher connects there) and
/// `307`-redirected to https for remote peers. Without [`Tls`] every
/// connection takes the plaintext path untouched.
async fn serve_sniffed(
	entity: AsyncEntity,
	stream: async_io::Async<std::net::TcpStream>,
	peer_addr: SocketAddr,
	tls: MaybeTls,
	guard: ServerGuard,
) -> Result {
	use stream_sniff::SecureProtocol;
	let (protocol, replay) = SecureProtocol::sniff(stream).await?;
	match protocol {
		SecureProtocol::Empty => Ok(()),
		SecureProtocol::PlainHttp => {
			if tls.is_active() && !peer_addr.ip().is_loopback() {
				let response =
					stream_sniff::https_redirect_response(replay.prefix())
						.unwrap_or_else(stream_sniff::tls_required_response);
				return stream_sniff::write_and_close(replay, response).await;
			}
			handle_connection(entity, replay, peer_addr, guard).await
		}
		SecureProtocol::Tls => {
			#[cfg(feature = "secure")]
			if let Some(server_tls) = tls.get() {
				let tls_stream = server_tls.accept(replay).await?;
				return handle_connection(entity, tls_stream, peer_addr, guard)
					.await;
			}
			debug!("TLS ClientHello on a plaintext listener, dropping");
			Ok(())
		}
	}
}

/// How much of a request body [`handle_connection`] reads per step, so the
/// buffer grows with the bytes that arrive rather than with the length the
/// client declared.
const BODY_READ_CHUNK: usize = 8192;

/// Handle a single HTTP connection: read the request, dispatch it,
/// and write the response. Generic over the transport so the sniffed
/// plaintext ([`stream_sniff::ReplayStream`]) and TLS streams land here alike.
async fn handle_connection<S>(
	entity: AsyncEntity,
	mut stream: S,
	peer_addr: SocketAddr,
	guard: ServerGuard,
) -> Result
where
	S: 'static
		+ Send
		+ Unpin
		+ futures_lite::AsyncRead
		+ futures_lite::AsyncWrite,
{
	use futures_lite::AsyncReadExt;
	use futures_lite::AsyncWriteExt;

	// Read the raw HTTP request headers (and possibly partial body)
	let mut buf = vec![0u8; 8192];
	let bytes_read = stream.read(&mut buf).await?;
	if bytes_read == 0 {
		return Ok(());
	}
	buf.truncate(bytes_read);

	// Read the rest of the body the headers declare, under the server's cap.
	//
	// The declared `content-length` is the one number a client picks, so it is
	// refused rather than believed, and the buffer grows as bytes ARRIVE rather
	// than to the declared length: a single `Content-Length: 2000000000` with no
	// body at all used to allocate and zero two gigabytes, which is one request
	// and an OOM kill on any box beet deploys to.
	let header_end = http_ext::find_header_end(&buf);
	if let Some(body_start) = header_end {
		let content_length = http_ext::parse_content_length(&buf[..body_start]);
		if content_length > guard.max_body_bytes() {
			let response = stream_sniff::payload_too_large_response(
				guard.max_body_bytes(),
			);
			return stream_sniff::write_and_close(stream, response).await;
		}
		let mut total_read = buf.len() - body_start;
		while total_read < content_length {
			let want = (content_length - total_read).min(BODY_READ_CHUNK);
			let at = buf.len();
			// `truncate` keeps the capacity, so a steady stream re-uses this
			// grow rather than reallocating per chunk
			buf.resize(at + want, 0);
			let read_count = stream.read(&mut buf[at..]).await?;
			buf.truncate(at + read_count);
			if read_count == 0 {
				break;
			}
			total_read += read_count;
		}
	}

	// Parse the raw HTTP request into our Request type, tagging the direct peer
	// address so a router middleware (eg analytics) can read the client address.
	let request = http_ext::parse_http_request(&buf)?
		.with_header_raw(PEER_ADDR_HEADER, &peer_addr.to_string());
	let is_head = request.method() == &HttpMethod::Head;

	// Dispatch through the router child, holding one of the server's permits
	// for as long as the build runs: a page build is megabytes while it is in
	// flight, so this is what bounds peak memory to
	// [`ServerLimits::max_in_flight`] times the cost of the dearest route,
	// whatever concurrency a client chooses. Taken HERE rather than around the
	// connection, so a client slow to send its request line queues nothing.
	let Some(permit) = guard.dispatch().await else {
		warn!("{peer_addr} waited out the dispatch limit, answering 503");
		return stream_sniff::write_and_close(
			stream,
			stream_sniff::service_unavailable_response(),
		)
		.await;
	};
	let response: Response = entity.exchange_child(request).await;
	drop(permit);

	// A `101 Switching Protocols` (a route returning `WebSocketUpgrade`) means we
	// write the handshake then keep the raw stream as a `Socket`, instead of
	// closing after the body.
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	if http_ext::is_websocket_response(&response) {
		return upgrade_connection(entity, stream, response).await;
	}

	let (parts, body) = response.into_parts();
	// a HEAD is a GET whose body is dropped: the route answered as for a GET,
	// and a body on the wire would corrupt the client's parse
	let body = match is_head {
		true => Body::Bytes(Bytes::new()),
		false => body,
	};

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
/// the side-port [`SocketServer::start_tungstenite`](crate::sockets) accept loop.
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
async fn upgrade_connection<S>(
	entity: AsyncEntity,
	stream: S,
	response: Response,
) -> Result
where
	S: 'static
		+ Send
		+ Unpin
		+ futures_lite::AsyncRead
		+ futures_lite::AsyncWrite,
{
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

/// The mini server behind an active [`Tls`]: https served, plaintext loopback
/// exempt from the redirect (the reload watcher path).
#[cfg(all(test, feature = "secure"))]
mod secure_test {
	use super::*;
	use crate::tls::test_client;

	#[beet_core::test]
	async fn serves_https_and_plaintext_loopback() {
		let server = HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let port = server.0.port.unwrap();
		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn((server, Tls::default(), children![
					exchange_ext::handler(|_| {
						Response::ok().with_body("secure hello")
					})
				]))
				.run();
		});
		time_ext::sleep_millis(300).await;
		let addr: SocketAddr = ([127, 0, 0, 1], port).into();

		// https: a TLS client trusting the dev cert
		let tls_stream = test_client::connect(addr).await.unwrap();
		test_client::raw_get(tls_stream, "/")
			.await
			.unwrap()
			.xpect_contains("200")
			.xpect_contains("secure hello");

		// plaintext from loopback stays served beside TLS
		let plain = async_io::Async::<std::net::TcpStream>::connect(addr)
			.await
			.unwrap();
		test_client::raw_get(plain, "/")
			.await
			.unwrap()
			.xpect_contains("200")
			.xpect_contains("secure hello");
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::Action;

	// -- integration test via shared suite --
	// (pure parse/serialise unit tests live with the shared helpers in
	// `crate::types::http_ext`.)

	/// A bound listener is data on its entity: a `port: 0` server records the
	/// OS-assigned address as [`Listening`], which is where a harness reads its
	/// url rather than being told it.
	#[beet_core::test]
	async fn records_listening() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn(
				HttpServer {
					port: Some(0),
					canonical: false,
					..default()
				}
				.with_backend(|entity, shutdown| {
					Box::pin(HttpServer::start_mini(entity, shutdown))
				}),
			)
			.id();
		super::super::http_server::tests::call_and_park(
			&mut app,
			entity,
			Request::get("/"),
		);
		app_ext::update_until_timeout(
			&mut app,
			|world| world.entity(entity).contains::<Listening>(),
			Duration::from_secs(5),
		)
		.await
		.xpect_true();
		let listening = *app.world().entity(entity).get::<Listening>().unwrap();
		listening.port().xpect_not_eq(0);
		listening
			.local_url()
			.xpect_eq(format!("http://127.0.0.1:{}", listening.port()));
	}

	/// A connection ends with its server entity: an idle keep-alive (a browser's
	/// preconnect) is cancelled by the despawn and its stream dropped, so the
	/// client's next read sees EOF and it retries on the rebuilt listener, rather
	/// than being answered by the torn-down entity with a 500.
	#[beet_core::test]
	async fn despawn_closes_connections() {
		use futures_lite::AsyncReadExt;
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let (server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let addr: SocketAddr = ([127, 0, 0, 1], server.port.unwrap()).into();
		let entity = app
			.world_mut()
			.spawn((server, on_spawn, children![exchange_ext::handler(|_| {
				Response::ok()
			})]))
			.id();
		// an idle connection: accepted, its task parked on the first read. The
		// accept loop is the one task in flight until the connection's joins it.
		let mut client = async_io::Async::<std::net::TcpStream>::connect(addr)
			.await
			.unwrap();
		app_ext::update_until_timeout(
			&mut app,
			|world| world.resource::<AsyncSpawner>().in_flight() == 2,
			Duration::from_secs(5),
		)
		.await
		.xpect_true();
		app.world_mut().entity_mut(entity).despawn();
		// the cancelled task drops its stream, so the client reads EOF
		let mut buf = [0u8; 1];
		let spawner = app.world().resource::<AsyncSpawner>().clone();
		AsyncRunner::poll_and_update(
			spawner,
			|| app.update(),
			client.read(&mut buf),
		)
		.await
		.unwrap()
		.xpect_eq(0);
	}

	/// Serve `handler` under `limits` on its own thread, returning the port a
	/// raw client can speak bytes at. Shared by the limit cases below, which
	/// assert on the wire rather than through a client that would normalise it.
	fn serve_with(
		limits: ServerLimits,
		handler: Action<Request, Response>,
	) -> u16 {
		let (server, on_spawn) =
			HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let port = server.port.unwrap();
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			app.world_mut()
				.spawn((server, on_spawn, limits, children![handler]));
			app.run();
		});
		port
	}

	/// Write `request` on a fresh connection and read the whole reply, which the
	/// mini server terminates by closing.
	async fn raw_exchange(port: u16, request: &[u8]) -> String {
		use futures_lite::AsyncReadExt;
		use futures_lite::AsyncWriteExt;
		let mut client = async_io::Async::<std::net::TcpStream>::connect((
			[127, 0, 0, 1],
			port,
		))
		.await
		.unwrap();
		client.write_all(request).await.unwrap();
		client.flush().await.unwrap();
		let mut reply = Vec::new();
		client.read_to_end(&mut reply).await.unwrap();
		String::from_utf8_lossy(&reply).to_string()
	}

	/// A declared body length over the cap is answered `413` before a byte of
	/// the body is read or allocated.
	///
	/// The read used to `resize` its buffer to whatever `content-length` said,
	/// so one request carrying `Content-Length: 2000000000` and no body at all
	/// allocated and zeroed two gigabytes. Live, `beet.org` had 1913 MB and no
	/// swap, which made that a one-request kill. The client here sends no body,
	/// so being answered at all is the proof.
	#[beet_core::test]
	async fn an_oversized_body_length_is_refused_unread() {
		let port = serve_with(
			ServerLimits {
				max_body_bytes: 64,
				..default()
			},
			exchange_ext::handler(|_| Response::ok()),
		);
		raw_exchange(
			port,
			b"POST / HTTP/1.1\r\nhost: x\r\ncontent-length: 2000000000\r\n\r\n",
		)
		.await
		.xpect_starts_with("HTTP/1.1 413");
	}

	/// A body inside the cap still arrives whole, so the incremental read that
	/// replaced the declared-length `resize` did not break the body path.
	#[beet_core::test]
	async fn a_body_inside_the_cap_arrives_whole() {
		let port = serve_with(
			ServerLimits::default(),
			exchange_ext::handler(|cx| {
				Response::ok().with_body(cx.take().body)
			}),
		);
		raw_exchange(
			port,
			b"POST / HTTP/1.1\r\nhost: x\r\ncontent-length: 11\r\n\r\nhello world",
		)
		.await
		.xpect_ends_with("hello world");
	}

	/// A wait for a permit that expires is answered `503` rather than hanging.
	///
	/// The valve on the one way the cap could wedge a server: a handler that
	/// calls its own server holds a permit while it waits for a second one, so
	/// the wait has to end in an answer. Here the first client holds the only
	/// permit for longer than the second will wait.
	#[beet_core::test]
	async fn a_request_that_waits_out_the_cap_is_answered() {
		let port = serve_with(
			ServerLimits {
				max_in_flight: 1,
				dispatch_timeout: Duration::from_millis(100),
				..default()
			},
			exchange_ext::handler_async(|_| async move {
				time_ext::sleep_millis(1500).await;
				Response::ok()
			}),
		);
		// zipped, not awaited in turn: a future does nothing until polled, so
		// awaiting the first to completion would hand the second a free permit
		let (first, second) = futures_lite::future::zip(
			raw_exchange(port, b"GET / HTTP/1.1\r\nhost: x\r\n\r\n"),
			raw_exchange(port, b"GET / HTTP/1.1\r\nhost: x\r\n\r\n"),
		)
		.await;
		// whichever wins the permit is answered `200` and the other `503`, so
		// assert on the pair rather than on an order the scheduler picks
		let mut answers = vec![first, second];
		answers.sort();
		let [served, refused] = answers.try_into().unwrap();
		served.xpect_starts_with("HTTP/1.1 200");
		refused.xpect_starts_with("HTTP/1.1 503");
	}

	/// At most [`ServerLimits::max_in_flight`] requests are dispatched at once,
	/// whatever concurrency the client chooses. This is what bounds peak memory,
	/// since a page build holds megabytes for as long as it runs: around a
	/// hundred concurrent requests for one unmatched path killed `beet.org`, and
	/// is what an ordinary vulnerability scanner does by default.
	#[beet_core::test]
	async fn the_dispatch_cap_bounds_concurrent_builds() {
		use futures_lite::AsyncReadExt;
		use futures_lite::AsyncWriteExt;
		const CAP: usize = 3;
		const CLIENTS: usize = 12;
		// one entry per handler currently inside the dispatch, and the count
		// each entry saw on the way in
		let live = Store::<Vec<()>>::default();
		let seen = Store::<Vec<usize>>::default();
		let (entered, observed) = (live.clone(), seen.clone());
		let port = serve_with(
			ServerLimits {
				max_in_flight: CAP,
				..default()
			},
			exchange_ext::handler_async(move |_| {
				let (entered, observed) = (entered.clone(), observed.clone());
				async move {
					entered.push(());
					observed.push(entered.len());
					// held long enough that an unbounded server would have
					// every client inside the handler at once
					time_ext::sleep_millis(50).await;
					entered.pop();
					Response::ok()
				}
			}),
		);

		// every request on the wire before any reply is read, so the server is
		// offered all of them at once
		let mut clients = Vec::new();
		for _ in 0..CLIENTS {
			let mut client = async_io::Async::<std::net::TcpStream>::connect((
				[127, 0, 0, 1],
				port,
			))
			.await
			.unwrap();
			client
				.write_all(b"GET / HTTP/1.1\r\nhost: x\r\n\r\n")
				.await
				.unwrap();
			client.flush().await.unwrap();
			clients.push(client);
		}
		for mut client in clients {
			let mut reply = Vec::new();
			client.read_to_end(&mut reply).await.unwrap();
			String::from_utf8_lossy(&reply).xpect_starts_with("HTTP/1.1 200");
		}
		// every client was served, and never more than the cap at a time
		seen.len().xpect_eq(CLIENTS);
		seen.get().into_iter().max().xpect_eq(Some(CAP));
	}

	#[cfg(feature = "ureq")]
	#[beet_core::test]
	async fn roundtrip() {
		super::super::http_server::test::test_server(
			HttpServer::start_mini_with_tcp,
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

		let server = HttpServer::new_test(HttpServer::start_mini_with_tcp);
		let port = server.0.port.unwrap();
		// records each landed socket entity so the test can assert the upgrade
		let landed = Store::<Vec<Entity>>::default();
		let captor = landed.clone();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			// a route that upgrades any request to a websocket
			app.world_mut()
				.spawn((server, children![exchange_ext::handler(|cx| {
					WebSocketUpgrade::from_request(&cx).into()
				})]));
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
