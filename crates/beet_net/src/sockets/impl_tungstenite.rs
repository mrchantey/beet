use crate::prelude::sockets::Message;
use crate::prelude::sockets::*;
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
pub async fn connect_tungstenite(url: impl AsRef<str>) -> Result<Socket> {
	// Parse URL to get host and port
	let parsed_url = url::Url::parse(url.as_ref())
		.map_err(|e| bevyhow!("Invalid URL: {}", e))?;
	let host = parsed_url
		.host_str()
		.ok_or_else(|| bevyhow!("URL missing host"))?
		.to_string();
	let port = parsed_url
		.port_or_known_default()
		.ok_or_else(|| bevyhow!("Cannot determine port"))?;

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
	let scheme = parsed_url.scheme();

	let (sink_boxed, stream_boxed): (
		Pin<Box<DynTungSink>>,
		Pin<Box<DynTungStream>>,
	) = match scheme {
		"ws" => {
			let (ws_stream, _resp) = client_async(url.as_ref(), tcp_stream)
				.await
				.map_err(|e| bevyhow!("WebSocket connect failed: {}", e))?;
			let (sink, stream) = ws_stream.split();
			(Box::pin(sink), Box::pin(stream))
		}
		"wss" => {
			#[cfg(feature = "native-tls")]
			{
				let connector = TlsConnector::new();
				let tls_stream = connector
					.connect(&host, tcp_stream)
					.await
					.map_err(|e| bevyhow!("TLS connect failed: {}", e))?;
				let (ws_stream, _resp) = client_async(url.as_ref(), tls_stream)
					.await
					.map_err(|e| bevyhow!("WebSocket connect failed: {}", e))?;
				let (sink, stream) = ws_stream.split();
				(Box::pin(sink), Box::pin(stream))
			}
			#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
			{
				let config = default_rustls_client_config()?;
				let (sink, stream) =
					rustls_connect(url.as_ref(), tcp_stream, &host, config)
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
		_ => {
			return Err(bevyhow!("Unsupported URL scheme: {}", scheme));
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

/// A tungstenite/bevy WebSocket server
///
/// This bevy system binds a TCP listener and spawns tasks to handle WebSocket connections.
/// Similar to [`start_hyper_server`] but for WebSockets.
pub(crate) fn start_tungstenite_server(
	In(entity): In<Entity>,
	query: Query<&SocketServer>,
	mut async_commands: AsyncCommands,
) -> Result {
	let server = query.get(entity)?;
	let addr = server.local_address();

	async_commands.run(async move |world| -> Result {
		let socket_addr: std::net::SocketAddr = addr
			.parse()
			.map_err(|e| bevyhow!("Invalid address {}: {}", addr, e))?;
		let listener = Async::<TcpListener>::bind(socket_addr)
			.map_err(|e| bevyhow!("Failed to bind to {}: {}", addr, e))?;

		let local_addr = listener
			.get_ref()
			.local_addr()
			.map_err(|e| bevyhow!("Failed to get local address: {}", e))?;

		info!("WebSocket server listening on ws://{}", local_addr);

		loop {
			let (stream, addr) = listener
				.accept()
				.await
				.map_err(|e| bevyhow!("Failed to accept connection: {}", e))?;

			trace!("New WebSocket connection from: {}", addr);

			// Spawn a new task for each connection
			// future can be discarded
			let _entity_fut = world.run_async(async move |world| {
				handle_connection(world.entity(entity), stream).await
			});
		}
	});

	Ok(())
}
async fn handle_connection(
	server: AsyncEntity,
	stream: Async<TcpStream>,
) -> Result {
	let ws_stream = accept_async(stream)
		.await
		.map_err(|e| bevyhow!("WebSocket handshake failed: {}", e))?;
	let (sink, stream) = ws_stream.split();

	let (sink_boxed, stream_boxed): (
		Pin<Box<DynTungSink>>,
		Pin<Box<DynTungStream>>,
	) = (Box::pin(sink), Box::pin(stream));

	// Map incoming tungstenite messages to our cross-platform Message
	let reader = stream_boxed.map(|res| match res {
		Ok(msg) => Ok(from_tung_msg(msg)),
		Err(err) => Err(bevyhow!("WebSocket receive error: {}", err)),
	});

	let writer = TungWriter {
		sink: Arc::new(Mutex::new(sink_boxed)),
	};

	server
		.spawn_child(Socket::new(reader, writer.clone()))
		.await;
	Ok(())
}

#[derive(Clone)]
struct TungWriter {
	sink: Arc<Mutex<Pin<Box<DynTungSink>>>>,
}

impl SocketWriter for TungWriter {
	fn clone_boxed(&self) -> Box<dyn SocketWriter> { Box::new(self.clone()) }

	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>> {
		let tmsg = to_tung_msg(msg);
		let sink = self.sink.clone();
		async move {
			let mut guard = sink.lock().await;
			guard
				.send(tmsg)
				.await
				.map_err(|e| bevyhow!("WebSocket send failed: {}", e))?;
			Ok(())
		}
		.boxed()
	}
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result<()>> {
		let sink = self.sink.clone();
		async move {
			let mut guard = sink.lock().await;
			match close {
				Some(cf) => {
					let frame = TungCloseFrame {
						code: close_code_from_u16(cf.code),
						reason: cf.reason.into(),
					};
					guard.send(TungMessage::Close(Some(frame))).await.map_err(
						|e| bevyhow!("WebSocket close send failed: {}", e),
					)?;
					// ensure the sink closes gracefully after sending close
					guard.close().await.map_err(|e| {
						bevyhow!("WebSocket close failed: {}", e)
					})?;
				}
				None => {
					guard.close().await.map_err(|e| {
						bevyhow!("WebSocket close failed: {}", e)
					})?;
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

	/// Tests for native-tls WSS: connect to a public echo server using the
	/// OS/platform certificate store.
	#[cfg(feature = "native-tls")]
	mod native_tls_tests {
		use super::super::connect_tungstenite;
		use super::Message;
		use futures::StreamExt;

		#[beet_core::test]
		async fn wss_native_tls_echo() {
			let url = "wss://echo.websocket.org";
			let mut socket = connect_tungstenite(url).await.unwrap();

			let payload = "beet-native-tls-test";
			socket.send(Message::text(payload)).await.unwrap();

			while let Some(item) = socket.next().await {
				match item.unwrap() {
					Message::Text(t) if t == payload => break,
					_ => continue,
				}
			}
			socket.close(None).await.ok();
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
