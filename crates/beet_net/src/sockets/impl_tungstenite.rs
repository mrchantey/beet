use crate::prelude::MaybeTls;
use crate::prelude::Scheme;
use crate::prelude::Url;
use crate::prelude::sockets::Message;
use crate::prelude::sockets::*;
use crate::prelude::stream_sniff;
use async_io::Async;
use async_lock::Mutex;
use async_tungstenite::accept_async;
use async_tungstenite::client_async;
use async_tungstenite::tungstenite::Error as TungError;
use async_tungstenite::tungstenite::Message as TungMessage;
use async_tungstenite::tungstenite::protocol::CloseFrame as TungCloseFrame;
use async_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungCloseCode;
use beet_core::prelude::*;
use bytes::Bytes;
use futures::FutureExt;
use futures::SinkExt;
use futures::StreamExt;
use futures::future::BoxFuture;

use std::net::TcpListener;
use std::net::TcpStream;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;

#[cfg(feature = "native-tls")]
use async_native_tls::TlsConnector;

#[cfg(feature = "rustls-tls")]
use futures_rustls::TlsConnector as RustlsConnector;
#[cfg(feature = "rustls-tls")]
use futures_rustls::rustls;

type DynTungSink =
	dyn futures::Sink<TungMessage, Error = TungError> + Send + Unpin;

type DynTungStream =
	dyn futures::Stream<Item = Result<TungMessage, TungError>> + Send + Unpin;

/// Connect to a WebSocket endpoint using async-tungstenite and return a cross-platform `Socket`.
///
/// This function:
/// - Establishes a client connection to `url`
/// - Splits the Tungstenite stream/sink
/// - Adapts the inbound `tungstenite::Message` stream into our cross-platform `Message`
/// - Wraps the sink in a writer that implements the `SocketWriter` trait
pub async fn connect_tungstenite(url: &Url) -> Result<Socket> {
	// split the authority; a bracketed ipv6 host is unwrapped for DNS/TLS
	let host = url
		.host()
		.ok_or_else(|| bevyhow!("URL missing host: {url}"))?
		.trim_matches(['[', ']'])
		.to_string();
	let port = url
		.port_or_default()
		.ok_or_else(|| bevyhow!("Cannot determine port: {url}"))?;
	let url_string = url.to_string();

	// Resolve DNS
	let host_clone = host.clone();
	let socket_addr = blocking::unblock(move || {
		(host_clone.as_str(), port)
			.to_socket_addrs()?
			.next()
			.ok_or_else(|| {
				std::io::Error::new(
					std::io::ErrorKind::NotFound,
					"cannot resolve address",
				)
			})
	})
	.await
	.map_err(|e| bevyhow!("DNS resolution failed: {}", e))?;

	// Connect TCP stream with async-io
	let tcp_stream = Async::<TcpStream>::connect(socket_addr)
		.await
		.map_err(|e| bevyhow!("TCP connect failed: {}", e))?;

	// Perform WebSocket handshake (with TLS if wss://)
	let (sink_boxed, stream_boxed): (
		Pin<Box<DynTungSink>>,
		Pin<Box<DynTungStream>>,
	) = match url.scheme() {
		Scheme::Ws => {
			let (ws_stream, _resp) =
				client_async(url_string.as_str(), tcp_stream)
					.await
					.map_err(|e| bevyhow!("WebSocket connect failed: {}", e))?;
			let (sink, stream) = ws_stream.split();
			(Box::pin(sink), Box::pin(stream))
		}
		Scheme::Wss => {
			#[cfg(feature = "native-tls")]
			{
				let connector = TlsConnector::new();
				let tls_stream = connector
					.connect(&host, tcp_stream)
					.await
					.map_err(|e| bevyhow!("TLS connect failed: {}", e))?;
				let (ws_stream, _resp) =
					client_async(url_string.as_str(), tls_stream)
						.await
						.map_err(|e| {
							bevyhow!("WebSocket connect failed: {}", e)
						})?;
				let (sink, stream) = ws_stream.split();
				(Box::pin(sink), Box::pin(stream))
			}
			#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
			{
				let config = default_rustls_client_config()?;
				let (sink, stream) = rustls_connect(
					url_string.as_str(),
					tcp_stream,
					&host,
					config,
				)
				.await?;
				(sink, stream)
			}
			#[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
			{
				return Err(bevyhow!(
					"WSS requires the native-tls or rustls-tls feature"
				));
			}
		}
		other => {
			return Err(bevyhow!("Unsupported URL scheme: {other}"));
		}
	};

	// Map incoming tungstenite messages to our cross-platform Message
	let reader = stream_boxed.map(|res| match res {
		Ok(msg) => Ok(from_tung_msg(msg)),
		Err(err) => Err(bevyhow!("WebSocket receive error: {}", err)),
	});

	let writer = TungWriter {
		sink: Arc::new(Mutex::new(sink_boxed)),
	};

	Ok(Socket::new(reader, writer))
}

/// Build a [`rustls::ClientConfig`] that trusts the Mozilla-curated root CAs
/// from the `webpki-roots` crate.
///
/// Uses an explicit `ring` crypto provider to avoid the runtime panic that
/// occurs when multiple providers (eg `ring` + `aws-lc-rs`) are enabled as
/// cargo features on `rustls`.
#[cfg(feature = "rustls-tls")]
#[allow(unused)]
pub(crate) fn default_rustls_client_config() -> Result<rustls::ClientConfig> {
	let provider = rustls::crypto::ring::default_provider();
	let mut root_store = rustls::RootCertStore::empty();
	root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
	rustls::ClientConfig::builder_with_provider(Arc::new(provider))
		.with_safe_default_protocol_versions()
		.map_err(|err| bevyhow!("rustls protocol version error: {}", err))?
		.with_root_certificates(root_store)
		.with_no_client_auth()
		.xok()
}

/// Perform a rustls TLS handshake over `stream` then upgrade to WebSocket.
///
/// Returns the boxed sink and stream halves ready for [`Socket`] construction.
#[cfg(feature = "rustls-tls")]
#[allow(unused)]
pub(crate) async fn rustls_connect(
	url: &str,
	stream: Async<TcpStream>,
	host: &str,
	config: rustls::ClientConfig,
) -> Result<(Pin<Box<DynTungSink>>, Pin<Box<DynTungStream>>)> {
	let connector = RustlsConnector::from(Arc::new(config));
	let server_name = rustls::pki_types::ServerName::try_from(host.to_owned())
		.map_err(|e| bevyhow!("Invalid server name '{}': {}", host, e))?;
	let tls_stream = connector
		.connect(server_name, stream)
		.await
		.map_err(|e| bevyhow!("TLS connect failed: {}", e))?;
	let (ws_stream, _resp) = client_async(url, tls_stream)
		.await
		.map_err(|e| bevyhow!("WebSocket connect failed: {}", e))?;
	let (sink, stream) = ws_stream.split();
	Ok((Box::pin(sink), Box::pin(stream)))
}

/// The native [`SocketServerFn`] backend: bind a TCP listener on the
/// [`SocketServer`] address and accept WebSocket connections until `shutdown`
/// resolves.
///
/// Installed by [`SocketServerPlugin`] via [`set_socket_server`]. Mirrors
/// [`start_mini_http_server`](crate::prelude::start_mini_http_server): it owns its
/// teardown by racing the accept loop against the shutdown signal.
///
/// Must be run on the main thread.
pub async fn start_tungstenite_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	let addr = entity
		.get::<SocketServer, _>(|server| server.local_address())
		.await?;
	let socket_addr: std::net::SocketAddr = addr
		.parse()
		.map_err(|e| bevyhow!("Invalid address {}: {}", addr, e))?;
	let listener = Async::<TcpListener>::bind(socket_addr)
		.map_err(|e| bevyhow!("Failed to bind to {}: {}", addr, e))?;
	start_tungstenite_server_with_tcp(entity, listener, shutdown).await
}

/// Accept WebSocket connections on a pre-bound listener until `shutdown` resolves.
///
/// Races the accept loop against the shutdown signal: when teardown signals, the
/// loop future is dropped, releasing the listener so the port closes.
pub(crate) async fn start_tungstenite_server_with_tcp(
	entity: AsyncEntity,
	listener: Async<TcpListener>,
	shutdown: OnceValueRx<()>,
) -> Result {
	let local_addr = listener
		.get_ref()
		.local_addr()
		.map_err(|e| bevyhow!("Failed to get local address: {}", e))?;

	// build the TLS acceptor (if any) before logging so the printed scheme is real
	let tls = MaybeTls::resolve(&entity).await?;
	info!(
		"WebSocket server listening on {}://{}",
		tls.ws_scheme(),
		local_addr
	);

	beet_core::exports::futures_lite::future::or(
		socket_accept_loop(entity, listener, tls),
		async move {
			shutdown.wait().await;
			Result::Ok(())
		},
	)
	.await
}

/// The accept loop: adopt each accepted connection as a child [`Socket`] entity.
/// Diverges (only the shutdown race in [`start_tungstenite_server_with_tcp`] ends
/// it).
async fn socket_accept_loop(
	entity: AsyncEntity,
	listener: Async<TcpListener>,
	tls: MaybeTls,
) -> Result {
	loop {
		let (stream, addr) = listener
			.accept()
			.await
			.map_err(|e| bevyhow!("Failed to accept connection: {}", e))?;

		trace!("New WebSocket connection from: {}", addr);

		let tls = tls.clone();
		entity
			.run_async_local(move |entity| async move {
				if let Err(err) =
					handle_connection(entity, stream, addr, tls).await
				{
					debug!("socket connection from {addr} failed: {err}");
				}
			})
			.await
			.ok();
	}
}

/// Classify one accepted connection and serve it: a TLS `ClientHello` is
/// accepted (when [`Tls`] is active) and handled inside the tunnel; plaintext
/// stays served, so native and embedded `ws://` peers (which have no
/// secure-context requirement) keep working beside a TLS-served browser. With
/// provided (real) certificates remote plaintext is rejected instead: the
/// generated dev cert grants no transport security, real certs might.
async fn handle_connection(
	server: AsyncEntity,
	stream: Async<TcpStream>,
	peer_addr: std::net::SocketAddr,
	tls: MaybeTls,
) -> Result {
	use crate::prelude::stream_sniff::SecureProtocol;
	let (protocol, replay) = SecureProtocol::sniff(stream).await?;
	match protocol {
		SecureProtocol::Empty => Ok(()),
		SecureProtocol::Tls => {
			#[cfg(feature = "secure")]
			if let Some(server_tls) = tls.get() {
				let tls_stream = server_tls.accept(replay).await?;
				return serve_socket(server, tls_stream, true).await;
			}
			debug!("TLS ClientHello on a plaintext socket listener, dropping");
			Ok(())
		}
		SecureProtocol::PlainHttp => {
			if tls.provided() && !peer_addr.ip().is_loopback() {
				return stream_sniff::write_and_close(
					replay,
					stream_sniff::tls_required_response(),
				)
				.await;
			}
			serve_socket(server, replay, false).await
		}
	}
}

/// Sniff the (possibly decrypted) http head: a websocket handshake becomes a
/// child [`Socket`], anything else (a browser `GET`) gets the landing page
/// instead of a failed handshake. Over TLS the landing page doubles as the
/// per-origin cert acceptance step: browsers show no acceptance UI for a
/// failed `wss://` handshake, so visiting the socket port over https once is
/// how the exception lands.
async fn serve_socket<S>(server: AsyncEntity, stream: S, tls: bool) -> Result
where
	S: 'static + Send + Unpin + futures::AsyncRead + futures::AsyncWrite,
{
	use crate::prelude::stream_sniff::SecureProtocol;
	let (protocol, replay) = SecureProtocol::sniff(stream).await?;
	match protocol {
		SecureProtocol::Empty => Ok(()),
		SecureProtocol::Tls => bevybail!("unexpected nested TLS handshake"),
		SecureProtocol::PlainHttp
			if stream_sniff::head_is_websocket_upgrade(replay.prefix()) =>
		{
			let ws_stream = accept_async(replay)
				.await
				.map_err(|e| bevyhow!("WebSocket handshake failed: {}", e))?;
			server.spawn_child(socket_from_ws_stream(ws_stream)).await;
			Ok(())
		}
		SecureProtocol::PlainHttp => {
			stream_sniff::write_and_close(
				replay,
				stream_sniff::socket_landing_response(tls),
			)
			.await
		}
	}
}

/// Build a cross-platform [`Socket`] from an already-handshaked tungstenite
/// stream by splitting it, mapping inbound `tungstenite::Message` to our
/// [`Message`], and wrapping the sink in a [`SocketWriter`].
///
/// Generic over the transport `S` so the side-port accept loop
/// ([`handle_connection`], an `Async<TcpStream>`) and the same-port upgrade
/// seams ([`socket_from_upgraded`] for the mini server, the hyper backend's
/// `HyperIo`) all land an identical `Socket`.
pub(crate) fn socket_from_ws_stream<S>(
	ws_stream: async_tungstenite::WebSocketStream<S>,
) -> Socket
where
	S: 'static + Send + Unpin + futures::AsyncRead + futures::AsyncWrite,
{
	let (sink, stream) = ws_stream.split();
	let (sink_boxed, stream_boxed): (
		Pin<Box<DynTungSink>>,
		Pin<Box<DynTungStream>>,
	) = (Box::pin(sink), Box::pin(stream));

	let reader = stream_boxed.map(|res| match res {
		Ok(msg) => Ok(from_tung_msg(msg)),
		Err(err) => Err(bevyhow!("WebSocket receive error: {}", err)),
	});
	let writer = TungWriter {
		sink: Arc::new(Mutex::new(sink_boxed)),
	};
	Socket::new(reader, writer)
}

/// Build a [`Socket`] from a raw stream whose `101 Switching Protocols`
/// handshake the HTTP backend already wrote (see `mini_http_server`/the hyper
/// backend). Wraps the stream with [`Role::Server`] without re-running the
/// handshake, the same-port counterpart to the side-port [`accept_async`].
pub(crate) async fn socket_from_upgraded<S>(stream: S) -> Socket
where
	S: 'static + Send + Unpin + futures::AsyncRead + futures::AsyncWrite,
{
	use async_tungstenite::WebSocketStream;
	use async_tungstenite::tungstenite::protocol::Role;
	let ws_stream =
		WebSocketStream::from_raw_socket(stream, Role::Server, None).await;
	socket_from_ws_stream(ws_stream)
}

#[derive(Clone)]
struct TungWriter {
	sink: Arc<Mutex<Pin<Box<DynTungSink>>>>,
}

impl SocketWriter for TungWriter {
	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>> {
		let tmsg = to_tung_msg(msg);
		let sink = self.sink.clone();
		async move {
			let mut guard = sink.lock().await;
			match guard.send(tmsg).await {
				Ok(_) => Ok(()),
				Err(TungError::ConnectionClosed | TungError::AlreadyClosed) => {
					// Expected during close handshake: peer already closed the connection
					debug!("WebSocket send skipped: connection already closed");
					Ok(())
				}
				Err(err) => Err(bevyhow!("WebSocket send failed: {}", err)),
			}
		}
		.boxed()
	}
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result> {
		let sink = self.sink.clone();
		async move {
			let mut guard = sink.lock().await;

			if let Some(cf) = close {
				let frame = TungCloseFrame {
					code: close_code_from_u16(cf.code),
					reason: cf.reason.into(),
				};
				match guard.send(TungMessage::Close(Some(frame))).await {
					Ok(_) => {}
					Err(
						TungError::ConnectionClosed | TungError::AlreadyClosed,
					) => {
						// do not even log a failed close message due to already closed
					}
					Err(e) => {
						bevybail!("WebSocket close send failed: {}", e);
					}
				}
			}
			match guard.close().await {
				Ok(_) => {}
				Err(TungError::ConnectionClosed | TungError::AlreadyClosed) => {
					// do not even log a failed close message due to already closed
				}
				Err(e) => {
					bevybail!("WebSocket close failed: {}", e);
				}
			}
			Ok(())
		}
		.boxed()
	}
}

fn from_tung_msg(msg: TungMessage) -> Message {
	match msg {
		TungMessage::Text(s) => Message::Text(s.to_string()),
		TungMessage::Binary(v) => Message::Binary(Bytes::from(v)),
		TungMessage::Ping(v) => Message::Ping(Bytes::from(v)),
		TungMessage::Pong(v) => Message::Pong(Bytes::from(v)),
		TungMessage::Close(opt) => {
			let cf = opt.map(|f| CloseFrame {
				code: close_code_to_u16(f.code),
				reason: f.reason.to_string(),
			});
			Message::Close(cf)
		}
		// Treat other message variants as no-op or "empty" binary
		_ => Message::Binary(Bytes::new()),
	}
}

fn to_tung_msg(msg: Message) -> TungMessage {
	match msg {
		Message::Text(s) => TungMessage::Text(s.into()),
		Message::Binary(b) => TungMessage::Binary(b),
		Message::Ping(b) => TungMessage::Ping(b),
		Message::Pong(b) => TungMessage::Pong(b),
		Message::Close(close) => {
			TungMessage::Close(close.map(|cf| TungCloseFrame {
				code: close_code_from_u16(cf.code),
				reason: cf.reason.into(),
			}))
		}
	}
}

fn close_code_to_u16(code: TungCloseCode) -> u16 {
	#[allow(unreachable_patterns)]
	match code {
		TungCloseCode::Normal => 1000,
		TungCloseCode::Away => 1001,
		TungCloseCode::Protocol => 1002,
		TungCloseCode::Unsupported => 1003,
		TungCloseCode::Status => 1005,
		TungCloseCode::Abnormal => 1006,
		TungCloseCode::Invalid => 1007,
		TungCloseCode::Policy => 1008,
		TungCloseCode::Size => 1009,
		TungCloseCode::Extension => 1010,
		TungCloseCode::Error => 1011,
		TungCloseCode::Restart => 1012,
		TungCloseCode::Again => 1013,
		TungCloseCode::Tls => 1015,
		_ => 1000,
	}
}

fn close_code_from_u16(code: u16) -> TungCloseCode { TungCloseCode::from(code) }

#[cfg(test)]
mod tests {
	use super::Message;
	use super::from_tung_msg;
	use super::to_tung_msg;
	use beet_core::prelude::*;
	use bytes::Bytes;

	#[beet_core::test]
	fn maps_messages_roundtrip() {
		let text = Message::text("hello");
		let bin = Message::binary(vec![1u8, 2, 3]);
		let ping = Message::ping(Bytes::from_static(b"p"));
		let pong = Message::pong(Bytes::from_static(b"q"));
		let close = Message::close(1000, "bye");

		let t_text = to_tung_msg(text.clone());
		let t_bin = to_tung_msg(bin.clone());
		let t_ping = to_tung_msg(ping.clone());
		let t_pong = to_tung_msg(pong.clone());
		let t_close = to_tung_msg(close.clone());

		from_tung_msg(t_text).xpect_eq(text);
		from_tung_msg(t_bin).xpect_eq(bin);
		from_tung_msg(t_ping).xpect_eq(ping);
		from_tung_msg(t_pong).xpect_eq(pong);
		// Close roundtrip may lose exact code mapping on older tungstenite, but should remain Close(..)
		matches!(from_tung_msg(t_close), Message::Close(_)).xpect_true();
	}

	/// Tests for native-tls: verify the tungstenite connect path works
	/// with the native-tls feature enabled using a local echo server.
	/// WSS/TLS is already thoroughly tested by `rustls_tls_tests`.
	#[cfg(feature = "native-tls")]
	mod native_tls_tests {
		use super::Message;
		use crate::sockets::Socket;
		use crate::sockets::echo_socket_server::EchoSocketServer;
		use futures::StreamExt;

		#[beet_core::test]
		async fn echo_local() {
			let server = EchoSocketServer::new().await;
			let mut socket =
				Socket::connect(&server.url().to_string()).await.unwrap();

			let payload = "beet-native-tls-test";
			socket.send(Message::text(payload)).await.unwrap();

			while let Some(item) = socket.next().await {
				match item.unwrap() {
					Message::Text(text) if text == payload => break,
					_ => continue,
				}
			}
			socket.close(None).await.ok();
		}
	}

	/// The [`SocketServer`] behind an active [`Tls`]: wss served, plaintext
	/// `ws://` peers (native, embedded) still served on the same port, and a
	/// plain browser `GET` answered with the landing page.
	#[cfg(feature = "secure")]
	mod secure_tests {
		use super::super::*;
		use crate::prelude::Tls;
		// explicit: the sockets enum must win over bevy's `Message` trait
		use crate::sockets::Message;
		use crate::tls::test_client;
		use async_tungstenite::client_async;
		use beet_core::prelude::*;

		#[beet_core::test]
		async fn serves_wss_plain_ws_and_landing() {
			let server = SocketServer::new_test();
			let port = server.0.port.unwrap();
			std::thread::spawn(move || {
				App::new()
					.add_plugins((
						MinimalPlugins,
						SocketServerPlugin::default(),
					))
					.spawn((server, Tls::default()))
					.run();
			});
			time_ext::sleep_millis(300).await;
			let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();

			// wss: a websocket handshake through the dev-cert TLS tunnel
			let tls_stream = test_client::connect(addr).await.unwrap();
			let (mut ws, _resp) =
				client_async(format!("ws://127.0.0.1:{port}"), tls_stream)
					.await
					.unwrap();
			ws.send(TungMessage::text("over wss")).await.unwrap();
			ws.close(None).await.ok();

			// plaintext ws to the same port keeps working (native/esp peers)
			let mut client = Socket::connect(format!("ws://127.0.0.1:{port}"))
				.await
				.unwrap();
			client.send(Message::text("plain")).await.unwrap();
			client.close(None).await.ok();

			// a browser GET (no upgrade) over TLS gets the landing page, the
			// per-origin cert acceptance step
			let tls_stream = test_client::connect(addr).await.unwrap();
			test_client::raw_get(tls_stream, "/")
				.await
				.unwrap()
				.xpect_contains("beet socket server")
				.xpect_contains("wss://");

			// and a plaintext GET gets its ws flavour
			let plain = Async::<TcpStream>::connect(addr).await.unwrap();
			test_client::raw_get(plain, "/")
				.await
				.unwrap()
				.xpect_contains("beet socket server")
				.xpect_contains("ws://");
		}
	}

	/// Tests for rustls-tls WSS: spin up a local WSS server backed by a
	/// self-signed certificate generated with `rcgen`, then connect using a
	/// [`rustls::ClientConfig`] that explicitly trusts that certificate.
	#[cfg(feature = "rustls-tls")]
	mod rustls_tls_tests {
		use super::super::DynTungSink;
		use super::super::DynTungStream;
		use super::super::rustls_connect;
		use async_io::Async;
		use async_tungstenite::accept_async;
		use async_tungstenite::tungstenite::Message as TungMessage;
		use beet_core::prelude::*;
		use futures::SinkExt;
		use futures::StreamExt;
		use futures_rustls::TlsAcceptor;
		use futures_rustls::rustls;
		use std::net::TcpStream;
		use std::pin::Pin;
		use std::sync::Arc;

		/// Generates a self-signed cert/key pair for `localhost` using rcgen,
		/// returning the DER cert (for client trust) and a rustls `ServerConfig`.
		fn make_test_server_config() -> (
			rustls::pki_types::CertificateDer<'static>,
			rustls::ServerConfig,
		) {
			let provider = rustls::crypto::ring::default_provider();
			let cert =
				rcgen::generate_simple_self_signed(vec!["localhost".into()])
					.expect("rcgen cert generation failed");
			let cert_der = rustls::pki_types::CertificateDer::from(
				cert.cert.der().to_vec(),
			);
			let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
				rustls::pki_types::PrivatePkcs8KeyDer::from(
					cert.signing_key.serialize_der(),
				),
			);
			let server_config =
				rustls::ServerConfig::builder_with_provider(Arc::new(provider))
					.with_safe_default_protocol_versions()
					.expect("failed to set protocol versions")
					.with_no_client_auth()
					.with_single_cert(vec![cert_der.clone()], key_der)
					.expect("failed to build rustls ServerConfig");
			(cert_der, server_config)
		}

		/// Builds a [`rustls::ClientConfig`] that trusts only the given DER cert.
		fn make_test_client_config(
			cert_der: rustls::pki_types::CertificateDer<'static>,
		) -> rustls::ClientConfig {
			let provider = rustls::crypto::ring::default_provider();
			let mut root_store = rustls::RootCertStore::empty();
			root_store.add(cert_der).expect("failed to add test cert");
			rustls::ClientConfig::builder_with_provider(Arc::new(provider))
				.with_safe_default_protocol_versions()
				.expect("failed to set protocol versions")
				.with_root_certificates(root_store)
				.with_no_client_auth()
		}

		/// Binds a local WSS echo server and returns its address.
		///
		/// The server is run on a background thread so it does not require
		/// Bevy's task pool and remains independent of the test executor.
		fn spawn_test_wss_server(
			acceptor: TlsAcceptor,
		) -> std::net::SocketAddr {
			// Bind synchronously so we can read the address before going async.
			let tcp_listener =
				std::net::TcpListener::bind("127.0.0.1:0").unwrap();
			let addr = tcp_listener.local_addr().unwrap();

			std::thread::spawn(move || {
				futures_lite::future::block_on(async move {
					// Convert to async-io listener (sets non-blocking mode).
					let listener =
						Async::new(tcp_listener).expect("async listener");
					loop {
						let Ok((stream, _)) = listener.accept().await else {
							break;
						};
						let acceptor = acceptor.clone();
						// Echo handler for a single connection (sequential is
						// fine for tests with one concurrent connection).
						let Ok(tls) = acceptor.accept(stream).await else {
							continue;
						};
						let Ok(mut ws) = accept_async(tls).await else {
							continue;
						};
						while let Some(Ok(msg)) = ws.next().await {
							match msg {
								TungMessage::Text(_)
								| TungMessage::Binary(_) => {
									ws.send(msg).await.ok();
								}
								TungMessage::Close(_) => break,
								_ => {}
							}
						}
					}
				});
			});

			addr
		}

		#[beet_core::test]
		async fn wss_rustls_tls_echo() {
			let (cert_der, server_config) = make_test_server_config();
			let acceptor = TlsAcceptor::from(Arc::new(server_config));
			let addr = spawn_test_wss_server(acceptor);

			// Let the background thread enter its accept loop.
			time_ext::sleep_millis(50).await;

			let client_config = make_test_client_config(cert_der);
			let url = format!("wss://localhost:{}", addr.port());

			let tcp = Async::<TcpStream>::connect(addr).await.unwrap();
			let (mut sink, mut stream): (
				Pin<Box<DynTungSink>>,
				Pin<Box<DynTungStream>>,
			) = rustls_connect(&url, tcp, "localhost", client_config)
				.await
				.unwrap();

			let payload = "beet-rustls-tls-test";
			sink.send(TungMessage::Text(payload.into())).await.unwrap();

			while let Some(Ok(msg)) = stream.next().await {
				match msg {
					TungMessage::Text(t) if t.as_str() == payload => break,
					_ => continue,
				}
			}
		}

		/// Verify [`default_rustls_client_config`] succeeds even when
		/// multiple crypto provider features are enabled simultaneously.
		#[beet_core::test]
		fn default_config_does_not_panic() {
			super::super::default_rustls_client_config().unwrap();
		}
	}
}
